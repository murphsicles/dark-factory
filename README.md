# Dark Factory — Rust → Zeta Autonomous Transpiler

**Turn any Rust crate into a Zeta package. Automatically.**

Dark Factory is the official Rust-to-Zeta transpiler, built by the [Zeta Foundation](https://zorbs.io). It parses Rust source code via `syn`, walks the AST, and emits equivalent Zeta source — preserving types, generics, traits, macros, extern blocks, and the full serde data model.

Used to produce all 46+ packages on [zorbs.io](https://zorbs.io), including:
- `@async/tokio` — Full async runtime, 373 files, 36k lines
- `@stdlib/regex` — Regular expression engine, 218 files, 69k lines
- `@data/serde` — Serialization framework, 24 files, 7.4k lines
- `@net/hyper` — HTTP/1.1 + HTTP/2 library
- `@crypto/ring` — Cryptography (AES, SHA, ECDSA, Ed25519)
- `@net/hyper`, `@net/reqwest`, `@net/tower` — Full web stack

## Quick Start

```bash
# Convert a single Rust file
df convert input.rs -o output.zeta

# Convert an entire crate directory
df crate path/to/crate

# Fetch a crate from crates.io and convert it
df fetch serde

# Full pipeline: fetch → convert → compile → publish
df pipeline clap --publish
```

## Installation

```bash
cargo install dark-factory
```

## Commands

| Command | Description |
|---------|-------------|
| `df convert <file>` | Convert a single `.rs` file to Zeta |
| `df crate <dir>` | Convert an entire crate directory |
| `df fetch <name>` | Fetch a crate from crates.io and convert |
| `df pipeline <name>` | Full pipeline: fetch → convert → compile → publish |
| `df rules` | List available AST rewrite rules |

## How It Works

Dark Factory operates in three passes:

1. **Parse** — Uses `syn` to parse Rust source into a full AST (File, Items, Expressions, Types, Patterns, Macros)
2. **Emit** — Walks the AST and emits Zeta source text. Each Rust construct has a corresponding `emit_*` function:
   - `emit_fn`, `emit_struct`, `emit_enum`, `emit_impl`, `emit_trait`
   - `emit_mod`, `emit_use`, `emit_const`, `emit_static`, `emit_type`
   - `emit_macro` — full `macro_rules!` preservation
   - `emit_foreign_mod` — `extern "C"` block handling
3. **Polish** — An 11-phase post-processor cleans up `proc_macro2` spacing artifacts (`::` → `::`, `assert ! (` → `assert!(`, turbofish, generics, doc URLs, etc.)

## Stats

Converted **46+ packages** totaling **~300,000+ lines of Zeta** with **zero unsupported items** across every crate. Tested on:
- `serde` (24 files, 7.4k lines)
- `regex` ecosystem (218 files, 69k lines)
- `tokio` (373 files, 36k lines)
- `clap_builder` (57 files, 11k lines)
- `rayon` + `rayon-core` (104 files, 17k lines)
- `crossbeam` ecosystem (42 files, 7.4k lines)
- `futures` ecosystem (409 files, 30k lines)
- And 35+ more

## License

MIT
