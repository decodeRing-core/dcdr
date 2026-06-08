# Plugin Development

decodeRing plugins are WebAssembly modules. The host communicates with them
over a fixed contract defined in `decodering-core`, so a plugin can be written
in any language that compiles to WASM and can speak that contract (for example
via an [Extism](https://extism.org) PDK).

This guide covers writing a new vault plugin. For compiling, manifests, and
configuration of plugins you already have, see the
[Compiling Plugins](../README.md#compiling-plugins) and
[Plugin Configuration](../README.md#plugin-configuration) sections of the
README.

## How the Contract Works

`decodering-core` is the single source of truth for the plugin contract. The
input and output types are plain Rust structs and enums annotated with
[`schemars`](https://crates.io/crates/schemars) (`#[derive(JsonSchema)]`).

Rather than hand-writing matching types in every plugin language, you generate
them:

1. **Core** defines the contract types.
2. The **CLI** emits those types as JSON Schema.
3. **quicktype** converts the JSON Schema into types in your plugin's language.

This keeps every plugin's types in sync with the host. When the contract
changes in core, you regenerate rather than edit by hand.

## Generating the Contract Types

### 1. Emit the JSON Schema

From the repository root:

```sh
cargo run --bin decodering-cli -- generate-schema
```

This writes the contract as JSON Schema (one or more `.json` files under
`schema/`).

### 2. Convert to Your Plugin's Language

Use [quicktype](https://quicktype.io) to turn the schema into typed source for
your plugin. Rust example, writing into the AWS plugin:

```sh
quicktype \
  --src-lang schema \
  --lang rust \
  --visibility public \
  schema/*.json \
  -o decodering-plugins/aws-rs/src/contract.rs
```

quicktype supports many targets (TypeScript, Go, Python, C#, and others), so
swap `--lang` for the language you are building the plugin in. Regenerate this
file whenever the core contract changes.

## The Contract

A plugin exports a fixed set of functions that the host calls by name. Each
takes a typed input and returns a typed output, serialized as JSON. Note that
the exported function names are not the same as the type names: `get_secret`
takes a `ReadInput`, `put_secret` takes a `WriteInput`, and so on.

| Exported function | Input           | Output            |
| ----------------- | --------------- | ----------------- |
| `get_secret`      | `ReadInput`     | `ReadOutput`      |
| `put_secret`      | `WriteInput`    | `WriteOutput`     |
| `delete_secret`   | `DeleteInput`   | `DeleteOutput`    |
| `destroy_secret`  | `DestroyInput`  | `DestroyOutput`   |
| `restore_secret`  | `RestoreInput`  | `RestoreOutput`   |
| `describe`        | `DescribeInput` | `DescribeOutput`  |
| `capabilities`    | none            | `Vec<Capability>` |

`capabilities` is the one function that takes no input. It returns the set of
`Capability` values the plugin supports, so the host knows which operations it
can route to this backend. Implement it to reflect exactly what your plugin
actually does.

> [!NOTE]
> Not every `Capability` corresponds to an exported function. `Taint` (and
> tainting in general) is enforced by the host, not the plugin, so there is no
> taint function to implement.

A few contract details worth calling out for plugin authors:

- **`SecretStatus`** (`Present`, `SoftDeleted`, `Destroyed`, `Disabled`,
  `NotFound`) is the normalized status decodeRing expects back, regardless of
  how the underlying vault represents state. Map the provider's native states
  onto these.
- **Version identifiers are opaque.** `VersionInfo.id` and the `version` fields
  are provider-defined strings (Vault uses `"3"`, AWS uses a `VersionId` UUID,
  and so on). Pass them through; do not assume a format.
- **Timestamps are RFC3339 strings** and are nullable. Return `null` when the
  provider does not expose the value rather than fabricating one.
- **`data` and `provider_hints` are arbitrary JSON.** `WriteInput.data` is any
  JSON payload, and `DescribeOutput.provider_hints` is opaque provider-native
  detail that callers must not assume a schema for.
- **`WriteInput.idempotency_token`** is supplied so writes can be retried
  safely. If the provider supports conditional or idempotent writes, use it.

## Configuration

Plugins read their configuration (addresses, regions, credentials) through the
host config interface. See
[Plugin Configuration](../README.md#plugin-configuration) for which values
belong in the manifest versus which should be passed as credentials through the
API. As a rule, non-sensitive values go in the manifest and secrets are passed
at runtime.

From the plugin's point of view, both sources arrive the same way: the host
merges API-supplied credentials into the manifest config before each call, so
your plugin reads everything (manifest values and injected credentials alike)
through the same config lookups.

> [!IMPORTANT]
> The host instantiates a fresh plugin instance on every call, with credentials
> injected at instantiation, then discards it. This per-call isolation is a
> deliberate security property. Do not rely on in-memory state persisting
> between calls; treat each invocation as stateless.

## Building and Installing

Once implemented, compile your plugin to WASM and install it like any other.
See [Compiling Plugins](../README.md#compiling-plugins) for the target setup,
build step, and manifest layout.
