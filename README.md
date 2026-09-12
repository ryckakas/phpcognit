# phpcognit

Cognitive complexity linter for PHP, shipped as a single static binary.

Cognitive complexity measures how hard code is to *read*, as opposed to cyclomatic
complexity, which measures how hard it is to *test*. A `switch` with twenty arms is
cyclomatically awful and cognitively fine; three nested `if`s are the reverse.
The metric is [SonarSource's](https://www.sonarsource.com/resources/cognitive-complexity/).

## Why another one

PHP already has several implementations — `TomasVotruba/cognitive-complexity` and
`Artemeon/cognitive-complexity` as PHPStan extensions, `Rarst/phpcs-cognitive-complexity`
as a PHP_CodeSniffer sniff, `ncac/php-cognitive-complexity` as a Composer CLI. All of
them run inside the PHP process of the project being analysed.

This one doesn't:

- **No PHP runtime required.** One binary scores PHP 7.x and 8.x alike, so a monorepo
  running several PHP versions uses one pinned tool rather than one per service.
- **Nothing added to the target repo.** No `composer require --dev`, no lockfile churn,
  no version conflict with the project's own PHPStan or PHP_CodeSniffer.
- **The scanned code is never executed.** Analysis is syntax-only — no autoloader, no
  reflection — which makes it safe to point at third-party or untrusted source.
- **Right-sized for the job.** Cognitive complexity is purely syntactic: nesting depth
  and operator sequences, no type or symbol resolution. Running a full semantic engine
  to compute it is more machinery than the metric needs.

## Install

No runtime, no toolchain, no Composer — one binary.

**Homebrew** (macOS and Linux)

```bash
brew install ryckakas/tap/phpcognit
```

**npm** — or run it without installing anything, which is usually what you want in CI

```bash
npx phpcognit --over 15 src/     # no install
npm install -g phpcognit         # or install it
```

**Windows**

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/ryckakas/phpcognit/releases/latest/download/phpcognit-installer.ps1 | iex"
```

**Manual** — download the archive for your platform from
[Releases](https://github.com/ryckakas/phpcognit/releases/latest), verify it against
the published SHA-256, and put `phpcognit` somewhere on your `PATH`. Builds are
provided for macOS (Apple Silicon and Intel), Linux (x86-64 and arm64), and
Windows (x86-64).

<details>
<summary>Install script, if you prefer it</summary>

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ryckakas/phpcognit/releases/latest/download/phpcognit-installer.sh | sh
```

</details>

Verify it works:

```bash
phpcognit --version
```

## Usage

```bash
phpcognit src/                  # fail on anything scoring above 15
phpcognit --over 10 src/        # stricter threshold
phpcognit --all src/            # print every function, ranked
phpcognit --format json src/    # machine-readable, for editors and CI
```

Output is `score  path:line  name`, ranked worst-first. Exit code is 1 if anything
exceeds the threshold, which is what makes it usable as a CI gate or pre-commit hook.

`--format json` emits the same data for tooling to consume. Note that the exit code
still reflects the threshold, so a consumer that only wants the data should read
stdout and ignore the exit status:

```json
{
  "threshold": 40,
  "breaches": 1,
  "findings": [
    {
      "path": "src/Controller/Component/FormSecurityComponent.php",
      "line": 135,
      "name": "getFormAccessibleFields",
      "score": 68
    }
  ]
}
```

## How the score is built

Three rules from the specification:

1. Shorthand that doesn't break reading flow is free — `??` scores nothing.
2. **+1** for each break in the linear flow.
3. **+nesting** when a flow-breaker sits inside other flow-breakers.

| Construct | Increment | Raises nesting |
| --- | --- | --- |
| `if`, ternary | +1 +nesting | yes |
| `elseif`, `else` | +1 flat | yes |
| `switch`, `match` | +1 +nesting (not per arm) | yes |
| `for`, `foreach`, `while`, `do` | +1 +nesting | yes |
| `catch` | +1 +nesting | yes |
| `try`, `finally` | — | no |
| `break N`, `continue N`, `goto` | +1 | no |
| Sequence of like boolean operators | +1 per run | no |
| Direct recursion | +1 | no |
| Closure, arrow fn, nested function | — | yes |

`elseif` takes a flat increment deliberately: a long `if`/`elseif` chain reads linearly,
so penalising it for depth would misrepresent it.

Boolean operators cost per *run*, not per operator — the cost is in the switching:

```php
$a && $b && $c              // +1  one run
$a && $b || $c              // +2  two runs
$a && $b && $c || $d || $e  // +3  three runs
$a && ($b || $c)            // +2  parentheses start a fresh run
```

## PHP specifics

The specification predates modern PHP, so these are decisions this tool makes:

- `match` (8.0) is treated as `switch`: one increment for the whole expression.
- `??` scores nothing — it is shorthand, and rule 1 applies.
- `and` / `or` normalise onto `&&` / `||` for run-counting; `xor` is its own operator.
- `elseif` and `else if` score identically, despite different parse shapes.
- `break N` / `continue N` are PHP's analogue of the specification's labelled break.
- Recursion is detected only through direct syntactic self-reference (`f()`,
  `$this->f()`, `self::f()`). Dispatch through a variable is not detectable without
  symbol resolution, and is not guessed at.

## Architecture

```
src/
├── complexity.rs   the scorer: parsed tree in, scores out. no I/O, no config
├── kinds.rs        every grammar node kind string, in one place
├── lib.rs          public API — the scorer is embeddable
└── main.rs         CLI: file discovery, ranking, exit codes
tests/
├── spec.rs         conformance table against the specification
└── grammar.rs      fails if a grammar upgrade renames a node out from under us
```

`complexity.rs` is deliberately free of filesystem and CLI concerns so it can be unit
tested directly and reused as a library.

## Development

Building from source needs Rust 1.90 or newer, via [rustup](https://rustup.rs). That
floor comes from `tree-sitter-language` rather than from anything this crate does, and
CI builds against it on every pull request so the number stays honest.

```bash
cargo build --release        # ./target/release/phpcognit
cargo install --path .       # or put it on your PATH
```

The same three gates run locally and in CI on every pull request:

```bash
cargo fmt --all -- --check                              # formatting
cargo clippy --all-targets --all-features -- -D warnings # lints, pedantic, warnings are errors
cargo test --all-features                                # spec conformance + grammar guard
```

Lint configuration lives in `Cargo.toml` under `[lints]` rather than in `#![deny]`
attributes, so editors, `cargo build`, and CI all see the same rules. Tests run on
Linux, macOS, and Windows — the cross-platform matrix is testing the product claim,
not decorating the badge.

### Releasing

Releases are cut by [`dist`](https://github.com/axodotdev/cargo-dist): pushing a
`v*` tag builds every target, generates the installers, and publishes a GitHub
Release. Preview what a tag would produce without pushing one:

```bash
dist plan
```

`.github/workflows/release.yml` is generated — edit `dist-workspace.toml` and re-run
`dist generate` rather than hand-editing the workflow, or the next `dist` run will
overwrite the changes.

## Licence

MIT
