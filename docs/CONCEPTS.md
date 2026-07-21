# Core Concepts

This page defines the vocabulary used throughout the CLI, API, and the rest of
the documentation. If you are new to decodeRing, read this before the
[Getting Started guide](GETTING-STARTED.md).

## The big picture

decodeRing sits between your applications and one or more external secrets
vaults (OpenBao, AWS Secrets Manager, …). Applications speak a single API —
the **OSL** — and decodeRing translates each request into calls against the
correct backend vault, applying authentication, authorization, and auditing
along the way. It stores its own metadata (who may access what) but is not
itself the vault of record for secret values; those live in the backends.

## Terms

**decodeRing / dcdr** — The server and its surrounding tooling. `dcdr` is the
short name used for the repository and CLI.

**OSL (Open Secrets Language)** — The REST API and request vocabulary for
secret operations (get, put, delete, destroy, restore, list, taint, describe).
It is the single interface applications use regardless of which backend vault
actually holds the secret.

**Node** — A single running `decodering-server` process.

**Cluster mode** — How state is stored and replicated. In **single** mode one
node uses one database (SQLite or PostgreSQL). In **raft** mode several nodes
replicate state through Raft consensus; each node keeps its own SQLite state
machine and RocksDB log.

**Backend / secret vault** — An external system that actually stores secret
values (OpenBao, AWS Secrets Manager). decodeRing talks to each backend through
a plugin.

**Plugin** — A WebAssembly module that implements the backend contract defined
in `decodering-core`. A plugin knows how to read/write/delete secrets in one
kind of backend. Plugins run sandboxed; see the [security model](SECURITY-MODEL.md).

**Plugin config** — The stored configuration for a backend instance, including
its type (e.g. `OpenBao`) and its credentials. Credentials are encrypted at
rest with the master key.

**Principal** — Any identity that can authenticate to decodeRing. Principals
have a _kind_, a _status_, and one or more credentials.

**User** — A principal representing a human operator, typically an
administrator. The first user is created during system initialization.

**App (application)** — A principal representing a workload/service that
consumes secrets through the OSL. Apps are granted access to specific secret
mappings.

**Credential** — The material a principal uses to prove identity. decodeRing
supports three credential kinds: **API key**, **TPM** (Trusted Platform
Module, hardware-backed), and **AWS IAM** (proving an AWS role identity).

**Token** — A bearer token issued after successful authentication and presented
on subsequent requests. A **root token** has full administrative rights and is
tied to system initialization; **short-term tokens** are time-bound tokens
issued to principals for ongoing operations.

**Secret mapping** — The link between a logical secret name that an app
requests through the OSL and the concrete location in a backend where that
secret lives. Mappings are what apps are granted access to.

**Grant** — An authorization record connecting a principal (app) to the secret
mappings it is allowed to use.

**Taint / untaint** — A secret mapping can be marked _tainted_ to flag it as
compromised or suspect without deleting it. Applications and operators can
check whether a mapping is tainted, and later untaint it once resolved.

**Capability** — An operation a backend supports (read, write, delete,
describe, destroy, restore). Not every backend supports every capability;
decodeRing exposes the capability set per backend so callers can discover what
is possible.

**Master key** — A 32-byte key held only in memory while the node is unsealed.
It encrypts backend credentials at rest. It is never written to disk in the
clear.

**Shamir shares / threshold** — At initialization the master key is split into
`n` shares using Shamir's Secret Sharing, with a threshold `k` of them required
to reconstruct it. Shares are distributed to separate operators so no single
person holds the key.

**Seal / unseal** — A freshly started node is _sealed_: it has no master key in
memory and cannot decrypt backend credentials. Operators _unseal_ it by
supplying at least `k` Shamir shares, which reconstructs the master key for
that process's lifetime.

**Audit log** — An append-only record of every action attempted — allowed,
denied, or errored — written in the same transaction as the action itself.

**Action** — The internal unit of a state change (e.g. "create app", "revoke
API key"). Every mutating OSL/management request maps to an action, which is
policy-checked, executed, and audited by a shared runner. In raft mode, actions
are applied deterministically across the cluster through consensus.

## How they fit together

1. An **operator** initializes the system: this generates the **master key**,
   splits it into **Shamir shares**, creates the first **user** and **root
   token**, and configures **plugins** for the backends.
2. On restart, operators **unseal** the node with `k` shares to restore the
   master key.
3. An operator enrolls **apps** with **credentials**, defines **secret
   mappings** to backend locations, and **grants** apps access to those
   mappings.
4. An **app authenticates** with its credential, receives a **short-term
   token**, and calls the **OSL** to get/put secrets.
5. decodeRing resolves the mapping, invokes the backend **plugin**, and
   returns the value, recording the whole exchange in the **audit log**.
