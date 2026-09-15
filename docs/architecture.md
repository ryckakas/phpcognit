# Architecture & development

[← Back to README](../README.md)

```text
src/
├── complexity.rs   the scorer: parsed tree in, scores out. no I/O, no config
├── baseline.rs     recorded scores, and what counts as a regression
├── kinds.rs        every grammar node kind string, in one place
├── lib.rs          public API — the scorer is embeddable
└── main.rs         CLI: file discovery, ranking, exit codes
tests/
├── spec.rs         conformance table against the specification
├── baseline.rs     grandfathering, including the new-function-in-old-file case
├── suppression.rs  marker parsing, and that a marker cannot leak past its function
├── cli.rs          the exit-code contract, including the silent-pass cases
└── grammar.rs      fails if a grammar upgrade renames a node out from under us
```

`complexity.rs` is deliberately free of filesystem and CLI concerns, so it can be unit
tested directly and reused as a library. Parsing is [tree-sitter](https://tree-sitter.github.io).

Building needs Rust 1.90 or newer via [rustup](https://rustup.rs) — a floor set by
`tree-sitter-language`, not by this crate, and one CI builds against on every pull
request so the number stays honest.

```bash
cargo build --release
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Lint configuration lives in `Cargo.toml` under `[lints]` rather than `#![deny]`
attributes, so editors, `cargo build` and CI all see the same rules. Tests run on Linux,
macOS and Windows.

**Releasing** is [`dist`](https://github.com/axodotdev/cargo-dist): pushing a `v*` tag
builds every target, generates the installers and publishes a GitHub Release. Preview
with `dist plan`. `.github/workflows/release.yml` is generated — edit
`dist-workspace.toml` and re-run `dist generate` rather than hand-editing it.

**The VS Code extension versions independently** of the CLI, and the two numbers are not
expected to match. It depends on `phpcognit` through npm on a caret range, so it picks up
CLI releases without a bump of its own; its version moves only when the extension itself
changes. That includes listing-only edits, because the Marketplace bundles the README
into the package and refuses a version it already holds. Publishing is manual — `vsce`
needs an Azure DevOps PAT, which now requires a paid subscription, so the VSIX is
uploaded by hand.
