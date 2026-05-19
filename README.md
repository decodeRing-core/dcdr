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

## **Getting Started**

1. Install the latest version of Rust (https://rust-lang.org/tools/install/)
2. RocksDB and SQLite bindings require a system C toolchain and LLVM/Clang development libraries to build. On Alpine: `apk add build-base clang-dev clang-libs llvm-dev`. On `Debian/Ubuntu: apt install build-essential clang libclang-dev`.
3. Clone repository
4. Create .env file with configuration. Adjust as needed.

```.env
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

```sh
rustup target list --installed
```

Install the wasm32-unknown-unknown target if you haven't already. The wasm32-unknown-unknown target means the plugin is not tied to any specific operating system or CPU architecture, it will run anywhere a WASM runtime is available.

```sh
rustup target add wasm32-unknown-unknown
```

From the decodering-plugins folder, run:

```sh
./build.sh
```

This compiles the plugins to WebAssembly and copies them into a plugins folder inside dcdr-rs (the directory where you cloned the repository). If you set PLUGIN_DIRECTORY to a different path, compile the plugins manually, see build.sh for details.
If the build succeeds, you should see a compiled/ folder containing **openbao-rust.wasm**.
Create a **manifests/** folder next to **compiled/** and add a file named **openbao-rust.yaml** with the following contents:

```yaml
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

```sh
dcdr-rs
  |- ...
  |- plugins
    |- compiled
      |- openbao-rust.wasm
    |- manifests
      |- openbao-rust.yaml
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

If this is a brand-new Raft cluster (i.e. no Raft state exists yet in RAFT\*LOG_DIR), you must initialize the cluster by issuing a request to [/raft/init](#raft-init)

---

## **Endpoints**

### **Raft**

#### Init {#raft-init}

Initialize a brand new raft cluster.

```
POST http://127.0.0.1:21001/raft/init
```

Request Body

```json
{
  "raft_init": []
}
```

#### Add Learner {#raft-add-learner}

Add a new learner to the cluster. First parameter is the ID of the learner and the second if the IP.

```
POST http://127.0.0.1:21001/raft/add-learner
```

Request Body

```json
[2, "127.0.0.1:21002"]
```

#### Metrics {#raft-metrics}

View raft node metrics

```
POST http://127.0.0.1:21001/raft/metrics
```

Request Body

```json
[]
```

#### Change Membership {#raft-change-membership}

Modify membership of the cluster. Add or remove nodes as needed.

```
POST http://127.0.0.1:21001/raft/change-membership
```

Request Body

##### Upgrade learners to voters.

```json
{
  "AddVoterIds": [1, 2]
}
```

##### Add voters with corresponding nodes.

```json
{
  "AddVoters": {
    "1": { "addr": "127.0.0.1:21001" },
    "2": { "addr": "127.0.0.1:21002" }
  }
}
```

##### Remove voters. Downgraded to learners

```json
{
  "RemoveVoters": [1, 2]
}
```

##### Replace all voters. The node of every new voter has to already be a learner.

```json
{
  "ReplaceAllVoters": [4, 5, 6]
}
```

##### Add nodes to membership, as learners. Does not replace existing nodes.

```json
{
  "AddNodes": {
    "1": { "addr": "127.0.0.1:21001" },
    "2": { "addr": "127.0.0.1:21002" }
  }
}
```

##### Add or replace nodes in membership config. Replaces existing nodes.

```json
{
  "SetNodes": {
    "1": { "addr": "127.0.0.1:21001" },
    "2": { "addr": "127.0.0.1:21002" }
  }
}
```

##### Remove learner nodes from membership.

```json
{
  "RemoveNodes": [1, 2]
}
```

##### Replace all learner nodes with a new set.

```json
{
  "ReplaceAllNodes": {
    "1": { "addr": "127.0.0.1:21001" },
    "2": { "addr": "127.0.0.1:21002" }
  }
}
```

##### Batch operations

```json
{
  "Batch": [
    {
      "AddNodes": {
        "4": { "addr": "127.0.0.1:21004" },
        "5": { "addr": "127.0.0.1:21005" }
      }
    },
    { "AddVoterIds": [4, 5] },
    { "RemoveVoters": [2] },
    { "RemoveNodes": [2] }
  ]
}
```
