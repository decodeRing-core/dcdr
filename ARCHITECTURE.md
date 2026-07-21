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
