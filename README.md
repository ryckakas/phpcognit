# phpcognit

Cognitive complexity linter for PHP, shipped as a single static binary.

Cognitive complexity measures how hard code is to *read*, where cyclomatic complexity
measures how hard it is to *test*. A `switch` with twenty arms is cyclomatically awful
and cognitively fine; three nested `if`s are the reverse. The metric is
[SonarSource's](https://www.sonarsource.com/resources/cognitive-complexity/).

No PHP runtime, no Composer entry, no PHPStan. The scanned code is never executed —
analysis is syntax-only, so it is safe to point at third-party source.

<details>
<summary><b>Why another one</b> — PHP already has four</summary>

`TomasVotruba/cognitive-complexity` and `Artemeon/cognitive-complexity` are PHPStan
extensions, `Rarst/phpcs-cognitive-complexity` is a PHP_CodeSniffer sniff, and
`ncac/php-cognitive-complexity` is a Composer CLI. All of them run inside the PHP
process of the project being analysed. This one doesn't:

- **No PHP-version coupling.** One pinned binary scores PHP 7.x and 8.x alike, so a
  monorepo running several versions needs one tool rather than one per service.
- **Nothing added to the target repo.** No `composer require --dev`, no lockfile churn,
  no version conflict with the project's own PHPStan or PHP_CodeSniffer.
- **Never executes what it scans.** No autoloader, no reflection.
- **Right-sized.** Cognitive complexity is purely syntactic — nesting and operator
  sequences, no type resolution. A full semantic engine is more machinery than the
  metric needs.

</details>

## Install

```bash
brew install ryckakas/tap/phpcognit    # macOS and Linux
npx phpcognit --over 15 src/           # or no install at all
```

<details>
<summary>Windows, manual download, install script</summary>

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/ryckakas/phpcognit/releases/latest/download/phpcognit-installer.ps1 | iex"
```

```bash
npm install -g phpcognit
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ryckakas/phpcognit/releases/latest/download/phpcognit-installer.sh | sh
```

Or take the archive for your platform from
[Releases](https://github.com/ryckakas/phpcognit/releases/latest), check it against the
published SHA-256, and put `phpcognit` on your `PATH`. Builds cover macOS (Apple
Silicon and Intel), Linux (x86-64 and arm64), and Windows (x86-64).

</details>

## Usage

```bash
phpcognit src/                  # fail on anything above 15
phpcognit --over 10 src/        # stricter
phpcognit --all src/            # every function, ranked
phpcognit --format json src/    # for editors and CI
```

Output is `score  path:line  Class::method`, worst first. A clean run prints nothing
and exits 0, so stdout stays usable in a pipeline.

Exit code is 1 on a breach — and also when the scan itself could not be trusted: an
unreadable path, or paths matching no PHP at all. A gate that silently passes because a
directory was renamed is worse than no gate.

<details>
<summary>JSON shape</summary>

The exit code still reflects the threshold, so a consumer that only wants the data
should read stdout and ignore the exit status.

```json
{
  "threshold": 40,
  "breaches": 1,
  "findings": [
    {
      "path": "src/Controller/Component/FormSecurityComponent.php",
      "line": 135,
      "name": "FormSecurityComponent::getFormAccessibleFields",
      "score": 68
    }
  ]
}
```

</details>

## Adopting on an existing codebase

Any codebase predating the tool has violations — one we tested against had 180. Nobody
refactors 180 functions to adopt a linter, so record them and gate on regressions:

```bash
phpcognit --write-baseline src/    # records today's findings, exits 0
phpcognit src/                     # fails only on new or worsened functions
```

Commit `.phpcognit-baseline.json`; it is picked up automatically wherever it exists, so
CI, hooks and your terminal agree without repeating flags.

Entries are keyed by function **name, not line**, which matters more than it sounds: a
grandfathered file does not become a hiding place. Add a complex new method to an
already-recorded file and it is reported, while the old ones around it stay accepted.
There is deliberately no ignore-by-file option — the accepted set stays a dated,
reviewable list rather than a glob that quietly widens.

## Suppressing one function

Some code is irreducibly branchy, and regenerating the whole baseline to accept one
deliberate case would re-record every other drift with it. Mark that function instead:

```php
// phpcognit-ignore: dispatch table; splitting it would obscure the mapping
public function dispatch(string $event): void
```

**The reason is mandatory.** A bare `// phpcognit-ignore` is refused, not obeyed — the
finding still reports and stderr says why. Suppression stays a decision someone wrote
down and a reviewer can see.

<details>
<summary>Where the marker may go</summary>

Above the declaration, inside its docblock, or trailing the signature line; attributes
in between are stepped over.

```php
public function dispatch(string $event): void // phpcognit-ignore: flat dispatch table
```

It has to be on the signature. A marker written inside the body is not a suppression,
so one comment can never silence the function it sits in.

Suppression hides a finding; it never changes a score. `--all` still shows the real
number.

</details>

<details>
<summary><b>How the score is built</b></summary>

Three rules from the specification: shorthand that doesn't break reading flow is free
(`??` scores nothing); **+1** for each break in linear flow; **+nesting** when a
flow-breaker sits inside other flow-breakers.

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

`elseif` takes a flat increment deliberately: a long chain reads linearly, so
penalising it for depth would misrepresent it.

Boolean operators cost per *run*, not per operator — the cost is in the switching:

```php
$a && $b && $c              // +1  one run
$a && $b || $c              // +2  two runs
$a && $b && $c || $d || $e  // +3  three runs
$a && ($b || $c)            // +2  parentheses start a fresh run
```

**PHP specifics** the specification predates:

- `match` (8.0) is treated as `switch`: one increment for the whole expression.
- `and` / `or` normalise onto `&&` / `||` for run-counting; `xor` is its own operator.
- `elseif` and `else if` score identically, despite different parse shapes.
- `break N` / `continue N` are PHP's analogue of the labelled break.
- Recursion is detected only through direct syntactic self-reference (`f()`,
  `$this->f()`, `self::f()`). Dispatch through a variable needs symbol resolution and
  is not guessed at.

</details>

<details>
<summary><b>Architecture and development</b></summary>

```
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
tested directly and reused as a library.

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
attributes, so editors, `cargo build` and CI all see the same rules. Tests run on
Linux, macOS and Windows — the matrix is testing the product claim, not decorating a
badge.

**Releasing** is [`dist`](https://github.com/axodotdev/cargo-dist): pushing a `v*` tag
builds every target, generates the installers and publishes a GitHub Release. Preview
with `dist plan`. `.github/workflows/release.yml` is generated — edit
`dist-workspace.toml` and re-run `dist generate` rather than hand-editing it.

</details>

## Licence

MIT
