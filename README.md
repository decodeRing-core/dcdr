<a name="top"></a>
[![decodeRing](https://org-web1.decodering.org/images/dcdr_banner.png)](https://decodering.org)

![Rust](https://img.shields.io/badge/Rust-1.95-orange)

> [!IMPORTANT]
> This is an alpha release and is not intended for production use. There are a number of features that need to be completed before the decodeRing server can be used in a production capacity.

⭐ Star us on GitHub — your support means a lot to us! 🙏😊

## About

decodeRing is an open-source security orchestration layer written in Rust that de-risks and accelerates secrets vault consolidation across clouds and vendors. decodeRing does this by implementing the [dcdr open standard](https://github.com/decodeRing-core/dcdr-standard) via RESTful API.

This allows developers to focus on coding instead of learning how to interact with multiple secrets back-ends. By abstracting away the complexity of the back-end secrets vaults decodeRing reduces friction for developers and provides SECOPS teams with the tools they need to consolidate their secrets landscape.

[Back to top](#top)

## Support & Community

- [GitHub Issues](https://github.com/decodeRing-core/dcdr-rs/issues) - report issues and make suggestions.
- [Community Forum](https://github.com/decodeRing-core/dcdr-rs/discussions) - ask questions, and start discussions!

To stay up-to-date with new features and improvements be sure to watch our repo!

## Architecture

The project is a Cargo workspace organized into the following crates:

| Workspace                                                                                     | Description                                                                                                                          | Depends on           |
| --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ | -------------------- |
| [decodering-core](https://github.com/decodeRing-core/dcdr-rs/tree/main/decodering-core)       | Shared abstractions across the codebase: plugins, actions, requests, responses, and other core logic.                                | —                    |
| [decodering-db](https://github.com/decodeRing-core/dcdr-rs/tree/main/decodering-db)           | Concrete storage backend implementations. Currently supports SQLite and PostgreSQL.                                                  | core                 |
| [decodering-raft](https://github.com/decodeRing-core/dcdr-rs/tree/main/decodering-raft)       | Raft consensus implementation for decodeRing, built on the [openraft](https://crates.io/crates/openraft) crate.                      | core, db             |
| [decodering-plugins](https://github.com/decodeRing-core/dcdr-rs/tree/main/decodering-plugins) | Plugins maintained by the decodeRing team that integrate with different vault backends.                                              | core                 |
| [decodering-auth](https://github.com/decodeRing-core/dcdr-rs/tree/main/decodering-auth)       | Authentication methods implementing the `AuthMethod` trait defined in core. Currently supports TPM, AWS IAM role, and API key.       | core                 |
| [decodering-server](https://github.com/decodeRing-core/dcdr-rs/tree/main/decodering-server)   | Implements the OSL (Open Secrets Language) REST API and handles Raft node management, system initialization, and ongoing operations. | core, db, raft, auth |
| [decodering-cli](https://github.com/decodeRing-core/dcdr-rs/tree/main/decodering-cli)         | Command-line tool for operators to interact with decodering-server without calling the REST API directly.                            | core                 |

## Quickstart

The fastest path to a running node — single mode, SQLite, no plugins:

```shell
# 1. Install Rust: https://rust-lang.org/tools/install/

# 2. Clone the repo
git clone https://github.com/decodeRing-core/dcdr-rs.git
cd dcdr-rs

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

For multi-node Raft clusters, plugin compilation, and PostgreSQL, see [Installation](#installation).

[Back to top](#top)

## Installation

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
