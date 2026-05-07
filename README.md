# **DecodeRing Rust Implementation**

## **Overview**
The project is organized into the following workspaces:
- decodering-cli
- decodering-core
- decodering-db
- decodering-plugins
- decodering-raft
- decodering-server

## decodering-core
Contains the abstractions shared across the codebase: plugins, actions, requests, responses, and other core logic. **This workspace must not depend on any other workspace in the project.**

## decodering-cli
A command-line tool that lets operators interact with decodering-server without calling the REST API directly.

## decodering-db
Concrete implementations of the storage backends. Currently supports SQLite and PostgreSQL. Depends on decodering-core.

## decodering-plugins
Plugins maintained by the DecodeRing team that integrate with different vault backends.

## decodering-raft
Concrete implementation of the Raft consensus protocol for DecodeRing, built on the openraft crate. Depends on decodering-core and decodering-db.

## decodering-server
Implements the OSL (Open Secrets Language) REST API and handles Raft node management, system initialization, and ongoing operations. Depends on decodering-core, decodering-db, and decodering-raft.

## **Getting Started**

- Install the latest version of Rust (https://rust-lang.org/tools/install/)
- RocksDB and SQLite bindings require a system C toolchain and LLVM/Clang development libraries to build. On *Alpine: apk add build-base clang-dev clang-libs llvm-dev*. On *Debian/Ubuntu: apt install build-essential clang libclang-dev*. 
- Clone repository
- Create .env file with configuration. Adjust as needed.
```
# Only `raft` is supported at this time.
STORAGE_MODE=raft
SERVER_LOG_OUTPUT=both
# For development only; use a persistent path in production.
SERVER_LOG_DIR="/tmp"
SERVER_LOG_PREFIX="decodering"
SERVER_LOG_MAX_FILES=0
# For development only; use a persistent path in production.
RAFT_LOG_DIR="/tmp"
RAFT_LOG_PREFIX="decodering"
TRACING_LEVEL=error,decodering=debug,extism=error,extism_pdk=error,tracing_actix_web=info
# The plugins folder must exist. In this example it lives inside the dcdr-rs folder.
PLUGIN_DIRECTORY="plugins"
```
#### **Compiling plugins**
Run:
```
  rustup target list --installed
```
Install wasm32-unknown-unknown target if you don't have it
```
  rustup target add wasm32-unknown-unknown
```
From the decodering-plugins folder, run:
```
./build.sh
```
This compiles the plugins to WebAssembly and copies them into a plugins folder inside dcdr-rs (the directory where you cloned the repository). If you set PLUGIN_DIRECTORY to a different path, compile the plugins manually, see build.sh for details.
If the build succeeds, you should see a compiled/ folder containing **openbao-rust.wasm**.
Create a **manifests/** folder next to **compiled/** and add a file named **openbao-rust.yaml** with the following contents:
```
wasm:
  - path: "plugins/compiled/openbao-rust.wasm"
allowed_hosts:
  - "127.0.0.1"
config:
  vault_addr: "http://127.0.0.1:8200"
  vault_token: "Your openbao vault token"
  kv_mount: "Your openbao kv mount"
```

The final layout should look like:
```
dcdr-rs
  |- ...
  |- Plugins
    |- compiled
      |- openbao-rust.wasm
    |- manifests
      |- openbao-rust.yaml
```

### **Run Nodes**

From the dcdr-rs directory, start a node with:
```
cargo run --bin decodering-server -- --id 1 --addr 127.0.0.1:21001
```

You can start additional nodes by incrementing the ID and port:
```
cargo run --bin decodering-server -- --id 2 --addr 127.0.0.1:21002
```

If this is a brand-new Raft cluster (i.e. no Raft state exists yet in RAFT_LOG_DIR), you must initialize the cluster by issuing a request to:
```
POST http://127.0.0.1:21001/raft/init
```

TODO
