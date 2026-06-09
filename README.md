<a name="top"></a>
[![decodeRing](https://org-web1.decodering.org/images/dcdr_banner.png)](https://decodering.org)

![Rust](https://img.shields.io/badge/built_with-Rust_1.96-orange) ![Rust MSRV](https://img.shields.io/badge/MSRV-1.95-orange) ![License](https://img.shields.io/badge/license-Apache--2.0-blue)
![Status](https://img.shields.io/badge/status-alpha-orange)

> [!IMPORTANT]
> This is an alpha release and is not intended for production use. There are a number of features that need to be completed before the decodeRing server can be used in a production capacity.

⭐ Star us on GitHub — your support means a lot to us! 🙏😊

## Contents

- [About](#about)
- [Support & Community](#support--community)
- [Architecture](#architecture)
- [Supported Integrations](#supported-integrations)
- [Quickstart](#quickstart)
- [Getting Started](#getting-started)
- [Installation](#installation)
  - [Compiling Plugins](#compiling-plugins)
  - [Plugin Configuration](#plugin-configuration)
  - [Running Nodes](#running-nodes)
  - [Build Errors](#build-errors)
- [API Reference](#api-reference)
- [Implementation Status](#implementation-status)
- [Plugin Development](#plugin-development)
- [Contributing](#contributing)
- [Security](#security)
- [License](#license)
- [Contacts](#contacts)

## About

decodeRing is an open-source security orchestration layer written in Rust that de-risks and accelerates secrets vault consolidation across clouds and vendors. decodeRing does this by implementing the [dcdr open standard](https://github.com/decodeRing-core/dcdr-standard) via RESTful API.

This allows developers to focus on coding instead of learning how to interact with multiple secrets back-ends. By abstracting away the complexity of the back-end secrets vaults decodeRing reduces friction for developers and provides SecOps teams with the tools they need to consolidate their secrets landscape.

[Back to top](#top)

## Support & Community

- [GitHub Issues](https://github.com/decodeRing-core/dcdr/issues) - report issues and make suggestions.
- [Community Forum](https://github.com/decodeRing-core/dcdr/discussions) - ask questions, and start discussions!

To stay up-to-date with new features and improvements be sure to watch our repo!

## Architecture

The project is a Cargo workspace organized into the following crates:

| Workspace                                                                                  | Description                                                                                                                          | Depends on           |
| ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------ | -------------------- |
| [decodering-core](https://github.com/decodeRing-core/dcdr/tree/main/decodering-core)       | Shared abstractions across the codebase: plugins, actions, requests, responses, and other core logic.                                | —                    |
| [decodering-db](https://github.com/decodeRing-core/dcdr/tree/main/decodering-db)           | Concrete storage backend implementations. Currently supports SQLite and PostgreSQL.                                                  | core                 |
| [decodering-raft](https://github.com/decodeRing-core/dcdr/tree/main/decodering-raft)       | Raft consensus implementation for decodeRing, built on the [openraft](https://crates.io/crates/openraft) crate.                      | core, db             |
| [decodering-plugins](https://github.com/decodeRing-core/dcdr/tree/main/decodering-plugins) | Plugins maintained by the decodeRing team that integrate with different vault backends.                                              | core                 |
| [decodering-auth](https://github.com/decodeRing-core/dcdr/tree/main/decodering-auth)       | Authentication methods implementing the `AuthMethod` trait defined in core. Currently supports TPM, AWS IAM role, and API key.       | core                 |
| [decodering-server](https://github.com/decodeRing-core/dcdr/tree/main/decodering-server)   | Implements the OSL (Open Secrets Language) REST API and handles Raft node management, system initialization, and ongoing operations. | core, db, raft, auth |
| [decodering-cli](https://github.com/decodeRing-core/dcdr/tree/main/decodering-cli)         | Command-line tool for operators to interact with decodering-server without calling the REST API directly.                            | core                 |

## Supported Integrations

**Secret vaults**

| Vault               | Status |
| ------------------- | :----: |
| OpenBao             |   ✅   |
| AWS Secrets Manager |   ✅   |

**Authentication methods**

| Method  | Status |
| ------- | :----: |
| API Key |   ✅   |
| TPM     |   ✅   |
| AWS IAM |   ✅   |

**Storage backends**

| Backend    | Status |
| ---------- | :----: |
| SQLite     |   ✅   |
| PostgreSQL |   ✅   |

## Quickstart

The fastest path to a running node — single mode, SQLite, no plugins:

```shell
# 1. Install Rust: https://rust-lang.org/tools/install/

# 2. Clone the repo
git clone https://github.com/decodeRing-core/dcdr.git
cd dcdr

# 3. Minimal .env
cat > .env <<'EOF'
CLUSTER_MODE=single
STORAGE_BACKEND=sqlite
DATABASE_URL="sqlite://decodering.db"
AUTO_MIGRATE=true
SERVER_LOG_OUTPUT=both
SERVER_LOG_DIR="/tmp"
SERVER_LOG_PREFIX="decodering"
SERVER_LOG_MAX_FILES=0
TRACING_LEVEL=error,decodering=debug
PLUGIN_DIR="plugins"
TPM_TRUST_DIR="/tmp"
EOF

# 4. Run a node
cargo run --bin decodering-server -- --id 1 --addr 127.0.0.1:21001
```

For a more detailed installation guide including running multi-node Raft
clusters, plugin compilation, and PostgreSQL, see [Installation](#installation).
For a full end-to-end walkthrough (cluster setup, applications, authentication,
and OSL secrets), see the [Getting Started guide](docs/GETTING-STARTED.md).

[Back to top](#top)

## Getting Started

For a complete end-to-end walkthrough (3-node Raft cluster, applications,
the three identity methods, and OSL put/get), see
[docs/GETTING-STARTED.md](docs/GETTING-STARTED.md).

## Installation

1. Install the latest version of [Rust](https://rust-lang.org/tools/install/).
2. RocksDB and SQLite bindings require a system C toolchain and LLVM/Clang development libraries to build:
   - **Alpine:** `apk add build-base clang-dev clang-libs llvm-dev`
   - **Debian/Ubuntu:** `apt install build-essential clang libclang-dev`
3. Clone the repository.
4. Create a `.env` file with your configuration and adjust as needed:

decodeRing runs in one of two modes. In single mode, set STORAGE_BACKEND to sqlite or postgres and point DATABASE_URL at it. In raft mode, STORAGE_BACKEND must be sqlite (the only backend currently supported with Raft); storage lives in RAFT_LOG_DIR and DATABASE_URL is ignored.

```shell
CLUSTER_MODE=single             # single | raft
STORAGE_BACKEND=postgres        # single: sqlite | postgres   raft: sqlite only (for now)

DATABASE_URL="postgresql://postgres:postgres@127.0.0.1:5432/testdb"  # required when CLUSTER_MODE=single

AUTO_MIGRATE=true               # default true; set false for controlled prod deploys

SERVER_LOG_OUTPUT=both
SERVER_LOG_DIR="/tmp"
SERVER_LOG_PREFIX="decodering"
SERVER_LOG_MAX_FILES=0

# SQLite storage is created in RAFT_LOG_DIR.
RAFT_LOG_DIR="/tmp"             # required when CLUSTER_MODE=raft
RAFT_LOG_PREFIX="decodering"    # required when CLUSTER_MODE=raft

TRACING_LEVEL=error,decodering=debug,extism=error,extism_pdk=error,tracing_actix_web=info

PLUGIN_DIR="plugins"

TPM_TRUST_DIR="/tmp"
```

### Compiling Plugins

Check your installed targets:

```shell
rustup target list --installed
```

Install the WASM targets if you haven't already. `wasm32-unknown-unknown` produces a plugin not tied to any OS or CPU architecture — it runs anywhere a WASM runtime is available. `wasm32-wasip1` adds WASI support, enabling access to system interfaces like the filesystem and environment variables.

```shell
rustup target add wasm32-unknown-unknown wasm32-wasip1
```

From the `decodering-plugins` folder, navigate into each vault plugin folder you want and run:

```shell
./build.sh
```

This compiles the plugins to WebAssembly and copies them into a `plugins` folder inside `dcdr` (the repository directory). If you set `PLUGIN_DIR` to a different path, compile the plugins manually — see `build.sh` for details.

On success you'll see a `compiled/` folder containing a `.wasm` file for each plugin you built. Each plugin requires a manifest file. Create a `manifests/` folder next to `compiled/` and add a `.yaml` file per plugin.

OpenBao example:

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

AWS Secrets Manager example:

```yaml
wasm:
  - path: "plugins/compiled/aws-rs.wasm"
allowed_hosts:
  - "secretsmanager.ap-southeast-2.amazonaws.com"
config:
  type: "AWS Secrets Manager"
  region: "ap-southeast-2"
```

Final layout:

```text
dcdr
  |- ...
  |- plugins
    |- compiled
      |- openbao-rs.wasm
      |- aws-rs.wasm
    |- manifests
      |- openbao-rs.yaml
      |- aws-rs.yaml
```

### Plugin Configuration

Each plugin reads a set of config keys. These can be supplied two ways:

- **Manifest YAML** — convenient, but stored in plaintext on disk. Use it for
  non-sensitive values only (addresses, regions, mount paths).
- **API** — credentials are passed at runtime and never written to the
  manifest. Supply them via `/system/init` (runs once, at cluster
  initialization) or `/system/plugin/config` (for updates after init).

> [!WARNING]
> Do not put secrets (`vault_token`, `aws_secret_access_key`, etc.) in the
> manifest YAML. Pass them through the API instead.

**OpenBao (`openbao-rs`)**

| Key           | Required | Default  | Recommended source |
| ------------- | :------: | -------- | ------------------ |
| `vault_addr`  |    ✅    | —        | manifest           |
| `kv_mount`    |    ❌    | `secret` | manifest           |
| `vault_token` |    ✅    | —        | API (credential)   |

**AWS Secrets Manager (`aws-rs`)**

| Key                     | Required | Default | Recommended source |
| ----------------------- | :------: | ------- | ------------------ |
| `region`                |    ✅    | —       | manifest           |
| `aws_access_key_id`     |    ✅    | —       | API (credential)   |
| `aws_secret_access_key` |    ✅    | —       | API (credential)   |
| `aws_session_token`     |    ❌    | —       | API (credential)   |

Credentials are keyed by plugin name under `plugins_credentials`. Example using
`/system/init`:

```sh
curl -X POST 'http://127.0.0.1:21001/system/init' \
  --header 'Content-Type: application/json' \
  --data '{
    "total_shares": 5,
    "threshold": 2,
    "plugins_credentials": {
      "openbao-rs": {
        "vault_token": "xxxx"
      },
      "aws-rs": {
        "aws_access_key_id": "xxxx",
        "aws_secret_access_key": "xxxx"
      }
    }
  }'
```

Because `/system/init` can only be run once, use `/system/plugin/config` to add
or rotate credentials afterward.

See [API Reference](#api-reference) for more detailed information about the available endpoints.

### Running Nodes

From the `dcdr` directory, start a node:

```shell
cargo run --bin decodering-server -- --id 1 --addr 127.0.0.1:21001
```

Start additional nodes by incrementing the ID and port:

```shell
cargo run --bin decodering-server -- --id 2 --addr 127.0.0.1:21002
```

### Build Errors

**`Unable to find libclang: ... Dynamic loading not supported`**

On Alpine, Rust defaults to statically-linked musl binaries, and static musl binaries can't `dlopen` shared libraries. Since `bindgen` loads `libclang.so` dynamically at build time, the build fails.

Fix by disabling static CRT linking so binaries are dynamically linked:

```shell
export RUSTFLAGS="-C target-feature=-crt-static"
```

To make this persistent, add it to `~/.cargo/config.toml`:

```toml
[build]
rustflags = ["-C", "target-feature=-crt-static"]
```

[Back to top](#top)

## API Reference

decodeRing exposes a REST API covering both the [OSL (Open Secrets Language)](https://github.com/decodeRing-core/osl) secret operations and system or management endpoints (initialization, plugin configuration, node management).

Once a node is running, it serves a work-in-progress OpenAPI specification and an interactive Swagger UI, at the address the node is listening on:

- OpenAPI spec (JSON): `http://<host>:<port>/api-docs/openapi.json`
- Swagger UI: `http://<host>:<port>/swagger-ui/`

For example, a node started with `--addr 127.0.0.1:21001` serves the spec at `http://127.0.0.1:21001/api-docs/openapi.json`. The specification is still being completed, so some endpoints may be missing or incomplete. For the operations currently supported, see [Implementation Status](#implementation-status).

## Implementation Status

All endpoints below require a root token or a short-term token. See the [OSL spec](https://github.com/decodeRing-core/osl) for more information.

| Capability            | Status |
| --------------------- | :----: |
| Get secret            |   ✅   |
| Put secret            |   ✅   |
| Destroy secret        |   ✅   |
| Delete secret         |   ✅   |
| Restore secret        |   ✅   |
| List secrets          |   ✅   |
| Taint secret          |   ✅   |
| Is secret tainted     |   ✅   |
| Untaint secret        |   ✅   |
| Get capabilities      |   ✅   |
| Secrets describe      |   ✅   |
| List applications     |   ✅   |
| List backends         |   ✅   |
| Secrets versions list |   ⬜   |
| Secret versions get   |   ⬜   |
| Issue credential      |   ⬜   |
| Renew credential      |   ⬜   |
| Revoke credential     |   ⬜   |
| Put rotation policy   |   ⬜   |
| Rotate secret         |   ⬜   |
| Put sync              |   ⬜   |
| Run sync              |   ⬜   |
| Get sync status       |   ⬜   |
| List syncs            |   ⬜   |
| Delete sync           |   ⬜   |

[Back to top](#top)

## Plugin Development

Plugins are WebAssembly modules and can be written in any language that
compiles to WASM. The host contract is defined in `decodering-core` and
generated into your plugin's language via JSON Schema. See
[PLUGIN-DEVELOPMENT.md](PLUGIN-DEVELOPMENT.md).

## Contributing

Contributions are welcome! To get started:

1. Fork the repository and create a feature branch.
2. Build and test: `cargo build && cargo test`.
3. Format and lint before submitting: `cargo fmt && cargo clippy --all-targets`.
4. Open a pull request describing your change.

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for full guidelines.

## Security

decodeRing handles secrets, so we take security seriously. **Please do not report security vulnerabilities through public GitHub issues.** Instead, email [security@decodering.org](mailto:security@decodering.org) (see [`SECURITY.md`](SECURITY.md) for our disclosure policy and supported versions).

As an alpha release, decodeRing has not yet undergone a formal security audit and should not be relied upon to protect production secrets.

[Back to top](#top)

## License

Licensed under the Apache License, Version 2.0.

[Back to top](#top)

## Contacts

For more details about our products, services, or any general information regarding the decodeRing Server, feel free to reach out to us. We are here to provide support and answer any questions you may have. Below are the best ways to contact our team:

- **Email**: Send us your inquiries or support requests at [support@decodering.org](mailto:support@decodering.org).
- **Website**: Visit the official decodeRing website for more information: [getdecodering.com](https://getdecodering.com/).

We look forward to assisting you and ensuring your experience with our product is successful and enjoyable!

[Back to top](#top)

```

```
