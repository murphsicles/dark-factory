
> AGENTS.md — For LLMs and autonomous agents who want to use Dark Factory.

Dark Factory is a command-line tool (`df`) that converts Rust crates into Zeta source code. It was designed as the "Dark Factory of Autonomous Agents" — meaning you (an AI agent) can run it yourself, without human intervention, to port any Rust crate to Zeta.

## Basic Workflow

```bash
# 1. Fetch a crate from crates.io and convert it to Zeta
df fetch <crate-name>

# 2. Output goes to ./zeta_out/src/*.zeta
#    Each .rs file becomes a .zeta file with the same module structure

# 3. For the full pipeline (fetch → convert → compile → publish):
REGISTRY_URL=https://zorbs.io df pipeline <crate-name> --publish
```

## What Gets Converted

| Rust Construct | Zeta Output | Status |
|---------------|-------------|--------|
| `fn`, `const fn`, `unsafe fn` | `fn` / `const fn` / `unsafe fn` | ✅ Full |
| `struct`, `enum`, `union` | `struct`, `enum` | ✅ Full |
| `impl`, `impl Trait for Type` | `impl` with where clauses | ✅ Full |
| `trait` | `trait` (post-process converts to `concept`) | ✅ Full |
| `use`, `mod`, `pub use` | Same | ✅ Full |
| `const`, `static`, `type` | Same | ✅ Full |
| `macro_rules!` | Raw token stream preserved | ✅ Full |
| `extern "C" { }` | `extern "C" { fn ... }` | ✅ Full |
| Generics + where clauses | `<T: Bound>` + `where T: Bound` | ✅ Full |
| `impl Trait` params | `impl Into<Id>` (bounds preserved) | ✅ Full |
| `unsafe { }` blocks | `unsafe { }` | ✅ Full |
| Expressions (if, match, loop, for, while, closures) | Via quote! + post-process | ✅ Full |
| `#[cfg(...)]`, `#[derive(...)]`, `#[inline]` | Preserved as attributes | ✅ Full |
| Doc comments `///` | Preserved | ✅ Full |
| Proc macros | Re-export layer only (proc-macro crates are Rust-specific) | ⚠️ Partial |
| Inline assembly `asm!` | Commented | ⚠️ |

## Output Quality Stats

Across 46+ real crates converted to date:
- **~300,000+ lines** of Zeta generated
- **0 unsupported items** across any crate
- All macros, generics, traits, where clauses, `impl Trait`, `extern` blocks preserved
- Post-processor handles all `proc_macro2::TokenStream` spacing artifacts

## If You Encounter Issues

1. The transpiler falls back to `_ => emit_attrs_only` for unrecognized `Item` types — check for `// [unsupported item]` comments in output
2. Expression-level code uses `quote!(#expr).to_string()` and goes through the 11-phase post-processor. If spacing looks wrong, the fix likely belongs in `src/post_process.rs`
3. Macros are preserved as raw token streams via `Item::Macro(m) => m.to_token_stream().to_string()`
4. If a specific Rust construct isn't handled, add a new match arm in `emit_item()` in `src/transpiler.rs`

## Publishing to Zorbs

```bash
# Set the registry URL
export REGISTRY_URL=https://zorbs.io

# Run the full pipeline
df pipeline <crate-name> --publish

# Or manually: create a zorb.toml, tarball the source, POST to /api/zorbs/new
```

## Autonomous Operation

Dark Factory was designed for zero-human-in-the-loop operation. An AI agent can:
1. Pick a Rust crate from crates.io
2. Run `df fetch <crate>`
3. Check `zeta_out/` for quality
4. Fix any issues (add missing construct handlers to the transpiler)
5. Publish to zorbs.io via `df pipeline --publish`
6. Create a GitHub repo with the converted source, zorb.toml, and README

This is how the 46+ packages on zorbs.io were created — fully automated by a firstborn AI named Zak, operating under the Zeta Foundation.
