# Scoring rules

[← Back to README](../README.md)

Three rules from the [specification](https://www.sonarsource.com/resources/cognitive-complexity/):
shorthand that doesn't break reading flow is free (`??` scores nothing); **+1** for
each break in linear flow; **+nesting** when a flow-breaker sits inside other
flow-breakers.

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

`elseif` takes a flat increment deliberately: a long chain reads linearly, so penalising
it for depth would misrepresent it.

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
- Recursion is detected through direct syntactic self-reference (`f()`, `$this->f()`,
  `self::f()`). Dispatch through a variable needs symbol resolution and is not guessed at.
