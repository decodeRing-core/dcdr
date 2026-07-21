# Architecture

decodeRing (`dcdr`) is a security orchestration layer that fronts multiple
secrets vaults behind a single REST API (the OSL — Open Secrets Language),
with optional Raft replication, pluggable authentication, and pluggable
storage.

## Crate dependency graph

The project is a Cargo workspace. Arrows point from a crate to the crates it
depends on.

```mermaid
flowchart TD
    core["decodering-core<br/>actions, traits, plugin & auth contracts"]
    db["decodering-db<br/>SQLite + Postgres repositories"]
    raft["decodering-raft<br/>openraft + RocksDB log store"]
    auth["decodering-auth<br/>TPM, AWS IAM, API key"]
    plugins["decodering-plugins<br/>openbao-rs, aws-rs (WASM)"]
    server["decodering-server<br/>REST API, node management"]
    cli["decodering-cli<br/>operator tool"]

    db --> core
    raft --> core
    raft --> db
    auth --> core
    plugins --> core
    cli --> core
    server --> core
    server --> db
    server --> raft
    server --> auth
```

## Deployment / component view

The server runs in one of two modes. Both expose the same REST/OSL API and
delegate secret access to sandboxed WASM plugins; they differ only in how
state is stored and replicated.

### Single mode

One node talks directly to a single SQLite **or** PostgreSQL database.

```mermaid
flowchart TB
    operator([Operator]) --> cli["decodering-cli"]
    app([Client app])

    cli -->|REST| server["decodering-server"]
    app -->|REST / OSL| server

    server --> store[("SQLite or PostgreSQL")]

    server --> openbao["openbao-rs"]
    server --> aws["aws-rs"]
    openbao --> bao[("OpenBao")]
    aws --> asm[("AWS Secrets Manager")]

    subgraph plugins["WASM plugins (extism)"]
        openbao
        aws
    end
```

### Raft mode

Multiple nodes replicate state via Raft. Each node keeps its **own** SQLite
state machine and RocksDB log store. Secret-backend access (plugins → vaults)
works exactly as in single mode and is omitted here for clarity.

```mermaid
flowchart TB
    clients([CLI / Client apps]) -->|REST / OSL| n1

    subgraph cluster["decodeRing Raft cluster"]
        n1["Server node 1"]
        n2["Server node 2"]
        n3["Server node 3"]
        n1 <-->|Raft| n2
        n1 <-->|Raft| n3
        n2 <-->|Raft| n3
    end

    subgraph pernode["Per-node storage (each node identical)"]
        sm[("SQLite<br/>state machine")]
        log[("RocksDB<br/>log store")]
    end

    n1 --> sm
    n1 --> log
```
