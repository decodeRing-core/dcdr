# Security Model

This document describes how decodeRing protects secrets and what an operator is
trusting when they run it. It is a design/trust-model document. To **report a
vulnerability**, follow [SECURITY.md](SECURITY.md) instead — do not open a
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
