# **DecodeRing Rust Implementation**

## **Overview**

The project is organized into the following workspaces:

#### **decodering-cli**

A command-line tool that lets operators interact with decodering-server without calling the REST API directly.

#### **decodering-core**

Contains the abstractions shared across the codebase: plugins, actions, requests, responses, and other core logic. _This workspace must not depend on any other workspace in the project._

#### **decodering-db**

Concrete implementations of the storage backends. Currently supports SQLite and PostgreSQL. Depends on decodering-core.

#### **decodering-plugins**

Plugins maintained by the DecodeRing team that integrate with different vault backends.

#### **decodering-raft**

Concrete implementation of the Raft consensus protocol for DecodeRing, built on the openraft crate. Depends on decodering-core and decodering-db.

#### decodering-server

Implements the OSL (Open Secrets Language) REST API and handles Raft node management, system initialization, and ongoing operations. Depends on decodering-core, decodering-db, and decodering-raft.

## **Installation**

1. Install the latest version of Rust (https://rust-lang.org/tools/install/)
2. RocksDB and SQLite bindings require a system C toolchain and LLVM/Clang development libraries to build. On Alpine: `apk add build-base clang-dev clang-libs llvm-dev`. On `Debian/Ubuntu: apt install build-essential clang libclang-dev`.
3. Clone repository
4. Create .env file with configuration. Adjust as needed.

```shell
CLUSTER_MODE=single # single | raft
STORAGE_BACKEND=postgres # sqlite | postgres

DATABASE_URL="postgresql://postgres:postgres@127.0.0.1:5432/testdb" # required if cluster mode is set to single

AUTO_MIGRATE=true   # default true; set false for controlled prod deploys

SERVER_LOG_OUTPUT=both
SERVER_LOG_DIR="/tmp"
SERVER_LOG_PREFIX="decodering"
SERVER_LOG_MAX_FILES=0

RAFT_LOG_DIR="/tmp" # required if cluster mode is set to raft
RAFT_LOG_PREFIX="decodering" # required if cluster mode is set to raft

TRACING_LEVEL=error,decodering=debug,extism=error,extism_pdk=error,tracing_actix_web=info

PLUGIN_DIR="plugins"

TPM_TRUST_DIR="/tmp"
```

#### **Compiling plugins**

Run:

```shell
rustup target list --installed
```

Install the wasm targets if you haven't already. The `wasm32-unknown-unknown` target means the plugin is not tied to any specific operating system or CPU architecture, it will run anywhere a WASM runtime is available. The `wasm32-wasip1` target includes WASI support, enabling access to system interfaces like the filesystem and environment variables.

```shell
rustup target add wasm32-unknown-unknown wasm32-wasip1
```

From the decoding-plugins folder, navigate into each vault plugin folder you want and run:

```shell
./build.sh
```

This compiles the plugins to WebAssembly and copies them into a plugins folder inside dcdr-rs (the repository directory). If you set PLUGIN_DIRECTORY to a different path, compile the plugins manually, see `build.sh` for details.
If the build succeeds, you should see a `compiled/` folder containing the `.wasm` files for each plugin you built.
Each plugin requires a manifest file. Create a `manifests/` folder next to `compiled/` and add a `.yaml` file for each plugin.

```yaml
wasm:
  - path: "plugins/compiled/openbao-rs.wasm"
allowed_hosts:
  - "127.0.0.1"
config:
  type: "OpenBao"
  vault_addr: "http://127.0.0.1:8200"
  kv_mount: "Your openbao kv mount"
```

```yaml
wasm:
  - path: "plugins/compiled/aws-rs.wasm"
allowed_hosts:
  - "secretsmanager.ap-southeast-2.amazonaws.com"
config:
  type: "AWS Secrets Manager"
  region: "ap-southeast-2"
```

The final layout should look like:

```shell
dcdr-rs
  |- ...
  |- plugins
    |- compiled
      |- openbao-rs.wasm
      |- aws-rs.wasm
    |- manifests
      |- openbao-rs.yaml
      |- aws-rs.yaml
```

### **Run Nodes**

From the dcdr-rs directory, start a node with:

```sh
cargo run --bin decodering-server -- --id 1 --addr 127.0.0.1:21001
```

You can start additional nodes by incrementing the ID and port:

```sh
cargo run --bin decodering-server -- --id 2 --addr 127.0.0.1:21002
```

### Build Errors

**`Unable to find libclang: ... Dynamic loading not supported`**

On Alpine, Rust defaults to statically-linked musl binaries, and static musl binaries can't `dlopen` shared libraries. Since `bindgen` loads `libclang.so` dynamically at build time, the build fails.

Fix by disabling static CRT linking so binaries are dynamically linked:

```sh
export RUSTFLAGS="-C target-feature=-crt-static"
```

To make this persistent, add it to `~/.cargo/config.toml`:

```toml
[build]
rustflags = ["-C", "target-feature=-crt-static"]
```

### **Getting Started**

[View examples](docs/GETTING-STARTED.md)

### **OSL**

**All endpoints below require a root token or a short-term token**

View [OSL spec](https://github.com/decodeRing-core/osl) for more information

Current implementation status

- [x] Get secret
- [x] Put secret
- [x] Destroy secret
- [x] Delete secret
- [x] Restore secret
- [x] List secrets
- [x] Taint secret
- [x] Is secret tainted
- [x] Untaint secret
- [x] Get capabilities
- [x] Secrets describe
- [ ] Secrets versions list
- [ ] Secret versions get
- [ ] Issue credential
- [ ] Renew credential
- [ ] Revoke credential
- [ ] Put rotation policy
- [ ] Rotate secret
- [ ] Put sync
- [ ] Run sync
- [ ] Get sync status
- [ ] List syncs
- [ ] Delete sync
- [x] List applications
- [x] List backends
