# phpcognit for VS Code

Cognitive complexity diagnostics for PHP, from the
[phpcognit](https://github.com/ryckakas/phpcognit) linter.

Functions above the threshold are marked as warnings on save and on open. What you see
in the editor is exactly what CI would fail on — baselined and suppressed findings stay
hidden, so the two never disagree.

## Settings

| Setting | Default | |
| --- | --- | --- |
| `phpcognit.enable` | `true` | Turn reporting off without uninstalling |
| `phpcognit.path` | `""` | Use a specific binary instead of the bundled one |
| `phpcognit.threshold` | `15` | Report functions scoring above this |

## Which binary it runs

In order: the `phpcognit.path` setting, then the copy bundled with this extension, then
`phpcognit` on your `PATH`. So a Homebrew install keeps whatever version you pinned,
and installing nothing still works.

If none is found you get one notification, once, with an option to dismiss it for good.

## Baselines

The scan runs from the workspace root, which is what lets `.phpcognit-baseline.json`
resolve — it records paths relative to the directory the tool runs in. Open a PHP file
from outside a workspace folder and no diagnostics are produced, because there is no
root to anchor the baseline to.

## Development

```bash
npm install
npm run compile
```

Then F5 in VS Code to launch an Extension Development Host.
