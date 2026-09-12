# phpcognit

**See which PHP functions are hard to read, while you're reading them.**

Marks functions whose cognitive complexity is too high, as you open and save files. No
PHP runtime, no Composer entry, nothing added to your project — the analyser is a Rust
binary that ships with the extension.

![A PHP method flagged with a cognitive complexity of 44](https://raw.githubusercontent.com/ryckakas/phpcognit/main/editors/vscode/images/diagnostic.png)

## Why cognitive complexity

It measures how hard code is to *read*, where cyclomatic complexity measures how hard
it is to *test*. A `switch` with twenty arms is cyclomatically awful and cognitively
fine; three nested `if`s are the reverse. The metric is
[SonarSource's](https://www.sonarsource.com/resources/cognitive-complexity/).

## What you get

- **Instant.** The analyser scans 600,000 lines in 1.5 seconds, so a single file is
  imperceptible.
- **Quiet on legacy code.** If your repo has a `.phpcognit-baseline.json`, everything
  recorded in it stays hidden. You see new complexity, not the decade you inherited.
- **Honest about suppressions.** A `// phpcognit-ignore: reason` comment hides a
  finding — but only with a reason attached.
- **Agrees with your pipeline.** The editor shows exactly what CI would fail on, so the
  two never contradict each other.
- **Never runs your code.** Analysis is syntax-only: no autoloader, no reflection.

## Settings

| Setting | Default | |
| --- | --- | --- |
| `phpcognit.threshold` | `15` | Report functions scoring above this |
| `phpcognit.enable` | `true` | Turn reporting off without uninstalling |
| `phpcognit.path` | `""` | Use a specific binary instead of the bundled one |

The threshold matches SonarSource's own default. Lower it for stricter review, raise it
while you dig out of a legacy codebase.

## Requirements

None. The binary ships with the extension and is fetched for your platform on first
use — macOS (Apple Silicon and Intel), Linux (x86-64 and arm64), Windows (x86-64).

If you already have `phpcognit` installed via Homebrew or npm, point `phpcognit.path`
at it and the extension will use your version instead.

## Adopting on an existing codebase

Any codebase that predates the tool has violations. Rather than being told to fix them
all, record them once and see only what's new:

```bash
phpcognit --write-baseline src/
```

Commit the resulting `.phpcognit-baseline.json`. From then on the editor stays quiet
about the old code and speaks up about the new — and so does CI.

## Links

[Repository and CLI documentation](https://github.com/ryckakas/phpcognit) ·
[Report an issue](https://github.com/ryckakas/phpcognit/issues) ·
[MIT licensed](https://github.com/ryckakas/phpcognit/blob/main/LICENSE)
