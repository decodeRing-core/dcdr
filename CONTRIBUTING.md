# Contributing to decodeRing

Thanks for your interest in contributing. decodeRing is in alpha, so
contributions, bug reports, and design feedback are all welcome.

> Found a security issue? Please do not open a public issue or pull request.
> See [SECURITY.md](SECURITY.md) for how to report it privately.

## Getting Started

1. Make sure you can build and run the project. See the
   [Installation](README.md#installation) section of the README for toolchain
   and system dependencies.
2. Fork the repository and create a branch for your change.
3. Make your change, keeping commits focused and descriptive.
4. Run the checks below, then open a pull request.

## Development Checks

Before opening a pull request, please run:

```sh
# Format and lint
cargo fmt --all
cargo clippy --all-targets

# Tests
cargo test
```

Pull requests should be formatted with `cargo fmt` and free of `clippy`
warnings. CI runs the same checks.

### Coverage (optional)

If you want to inspect test coverage locally, install
[`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov) and run:

```sh
cargo llvm-cov --html --open
```

Coverage is a guide, not a gate. We do not enforce a coverage threshold.

## Workspace Layout and Boundaries

decodeRing is a Cargo workspace. See the
[Architecture](README.md#architecture) section for what each crate does and how
they depend on one another. One rule matters most when contributing:

- **`decodering-core` must not depend on any other workspace crate.** Shared
  abstractions and traits (for example `AuthMethod`) live in core; concrete
  implementations live in the crates that depend on it. Adding a dependency
  from core to another workspace crate will be rejected.

When adding a new backend, plugin, or auth method, implement the relevant trait
from core rather than reaching across crates.

## Pull Requests

- Keep each pull request focused on a single change where possible.
- Describe what the change does and why. Link any related issue.
- Update documentation (README, examples, config notes) when behavior changes.
- Make sure the development checks above pass.

A maintainer will review your pull request and may request changes. Thanks in
advance for your patience while the project is still young.

## Reporting Bugs and Requesting Features

- [GitHub Issues](https://github.com/decodeRing-core/dcdr/issues) for bugs
  and feature requests.
- [Community Forum](https://github.com/decodeRing-core/dcdr/discussions) for
  questions and open-ended discussion.

When reporting a bug, please include the version or commit, your platform, the
relevant configuration (with secrets removed), and steps to reproduce.
