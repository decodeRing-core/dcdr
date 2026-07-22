# Security Model

This document describes how decodeRing protects secrets and what an operator is
trusting when they run it. It is a design/trust-model document. To **report a
vulnerability**, follow [SECURITY.md](../SECURITY.md) instead — do not open a
public issue.

> decodeRing is alpha software and has not had a formal security audit. See
> [Limitations](#limitations-and-non-goals) before relying on it.

## What decodeRing is (and isn't) responsible for

decodeRing is an orchestration and access-control layer in front of external
secret vaults. Secret _values_ live in the backends (OpenBao, AWS Secrets
Manager). decodeRing stores metadata principals, credentials, secret
mappings, grants, plugin configuration, and the audit log, and mediates every
access to the backends. The security of a deployment therefore depends on both
decodeRing and the backends it fronts.

## Trust boundaries

| Boundary               | What crosses it                     | Protection                                                                    |
| ---------------------- | ----------------------------------- | ----------------------------------------------------------------------------- |
| Client ↔ server        | OSL / management requests, tokens   | Authentication + authorization on every request; audit of every attempt       |
| Server ↔ backend vault | Secret values, backend credentials  | Credentials decrypted in memory only; calls made from a sandboxed plugin      |
| Server ↔ plugin (WASM) | Per-call credentials and secret I/O | Extism sandbox; fresh plugin instance per call; host-declared `allowed_hosts` |
| Node ↔ node (raft)     | Replicated actions and log entries  | Deterministic state machine; audit replicated with state                      |
| Operator ↔ master key  | Shamir shares                       | Key never persisted in clear; split across operators                          |

# The master key and seal / unseal

At initialization decodeRing generates a random 32-byte **master key** and
splits it into `n` shares using Shamir's Secret Sharing, requiring a threshold
`k` (`2 ≤ k ≤ n`, `n ≤ 10`) to reconstruct. A SHA-256 hash of the key is stored
so the server can verify that a reconstruction from submitted shares is correct.

The master key exists **only in process memory** while a node is unsealed. It is
held in a write-once, zero-on-drop container (`OnceLock` + `Zeroizing`), so it
is set exactly once per process lifetime and wiped when the process exits. It is
never written to disk in the clear.

A freshly started node is **sealed**: no master key, and it cannot decrypt
backend credentials. Operators unseal it by submitting at least `k` shares.
Because shares are distributed to separate people, no single operator can unseal
a node alone — this is the primary defense against a single compromised operator
or a stolen disk.

## Credentials at rest

Backend/plugin credentials are encrypted with the master key using
**AES-256-GCM**. Each stored blob is laid out as a 12-byte random nonce followed
by the ciphertext and its 16-byte authentication tag. The backend's name is
passed as additional authenticated data (AAD), binding each ciphertext to its
context so a blob cannot be silently swapped between backends without detection.

Because encryption depends on the master key, a sealed node, or a stolen
database without the Shamir shares cannot recover any backend credentials.

## Plugin sandboxing

Backend integrations run as WebAssembly modules via
[extism](https://extism.org/), not as native code with host privileges. Two
properties matter for security:

- **Per-call isolation.** A fresh plugin instance is created for each backend
  call, with that call's credentials injected into the instance configuration.
  Instances are not reused across calls, limiting the blast radius of a
  misbehaving or compromised plugin.
- **Declared network reach.** A plugin's manifest declares its `allowed_hosts`;
  it can only reach the endpoints it is configured for.

## Authentication

Every principal authenticates through an `AuthMethod` implementation. Three
methods ship today, each declaring its capabilities so the server knows the
exact flow:

- **API key** — a bearer secret; simplest flow (no challenge/activation step).
- **TPM (Trusted Platform Module)** — hardware-backed. Uses a challenge/response
  and an activation step so that possession of the TPM, not just knowledge of a
  secret, is required.
- **AWS IAM** — proves an AWS role identity, so workloads already running under
  an IAM role authenticate without a long-lived shared secret.
  The method interface separates _resolving_ which credential a proof refers to
  from _verifying_ the proof, and it never calls back into the host, all inputs
  are passed in explicitly, which keeps the trusted surface small.

## Authorization and tokens

Successful authentication yields a token. A **root token**, established at
system initialization, carries full administrative rights. **Short-term tokens**
are issued to principals for ongoing operations and are time-bound, limiting the
window in which a leaked token is useful. Access to a given secret is governed by
**grants** linking an app to specific **secret mappings**; an app can only reach
the mappings it has been granted.

## Auditing

Every action, allowed, denied, or errored is recorded in an append-only
audit log. Auditing is not best-effort: the audit entry is written in the same
database transaction as the action, so a committed change always has a
corresponding audit record, and a denied or failed attempt is still recorded. In
raft mode the audit log is replicated with the rest of the state machine.

## Integrity in a cluster

In raft mode, state changes are applied as deterministic actions through Raft
consensus, so all nodes converge on the same state and audit history. A single
node cannot unilaterally rewrite committed state.

## Defensive coding posture

The workspace lints deny several classes of runtime failure in library and
server code. This reduces the chance that malformed input crashes a node or
triggers undefined-shaped error paths.

## Limitations and non-goals

decodeRing is alpha. Known caveats an evaluator should weigh:

- **No formal audit yet.** The design has not been independently reviewed.
  Do not use it to protect production secrets.
- **Backend trust is inherited.** decodeRing is only as strong as the vaults it
  fronts and the credentials configured for them.
- **Transport security is deployment-provided.** Run nodes behind TLS; the
  server does not assume a hostile local network for you.
- **Operator trust.** An operator holding a threshold of Shamir shares, or the
  root token, can unseal and administer the system. Distribute shares and guard
  the root token accordingly.
- **Feature gaps.** Secret versioning, rotation, credential issuance/renewal,
  and sync are not yet implemented;

If you find a security issue, report it privately per [SECURITY.md](../SECURITY.md).
