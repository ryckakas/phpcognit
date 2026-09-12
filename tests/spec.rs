//! Conformance against the `SonarSource` cognitive complexity specification.
//!
//! Each case states the increments it expects so a failure says which rule
//! broke, not just which number moved.

fn score(body: &str) -> u32 {
    let source = format!("<?php\nfunction target() {{\n{body}\n}}\n");
    let mut parser = phpcognit::parser().expect("PHP grammar should load");

    phpcognit::analyze_source(&mut parser, &source)
        .into_iter()
        .find(|finding| finding.name == "target")
        .unwrap_or_else(|| panic!("no finding produced for:\n{source}"))
        .score
}

fn assert_scores(cases: &[(&str, u32)]) {
    for (body, expected) in cases {
        assert_eq!(score(body), *expected, "scoring:\n{body}");
    }
}

#[test]
fn operator_sequences_cost_per_run_not_per_operator() {
    assert_scores(&[
        // if +1, one `&&` run +1
        ("if ($a && $b && $c) { echo 1; }", 2),
        // if +1, `&&` run then `||` run +2
        ("if ($a && $b || $c) { echo 1; }", 3),
        // if +1, three runs: && -> || -> &&
        ("if ($a && $b && $c || $d || $e && $f) { echo 1; }", 4),
        // parentheses start a fresh sequence: if +1, `&&` +1, `||` +1
        ("if ($a && ($b || $c)) { echo 1; }", 3),
        // `and`/`or` normalise onto `&&`/`||`
        ("if ($a and $b and $c) { echo 1; }", 2),
    ]);
}

#[test]
fn null_coalescing_is_shorthand_and_costs_nothing() {
    assert_scores(&[("if ($a ?? $b) { echo 1; }", 1)]);
}

#[test]
fn nesting_compounds() {
    assert_scores(&[
        // if +1
        ("if ($a) { echo 1; }", 1),
        // if +1, inner if +1+1
        ("if ($a) { if ($b) { echo 1; } }", 3),
        // if +1, inner if +2, innermost +3
        ("if ($a) { if ($b) { if ($c) { echo 1; } } }", 6),
        // foreach +1, if +2
        ("foreach ($xs as $x) { if ($x) { echo 1; } }", 3),
    ]);
}

#[test]
fn else_branches_take_a_flat_increment() {
    assert_scores(&[
        // if +1, elseif +1, else +1 - no nesting penalty on the chain
        (
            "if ($a) { echo 1; } elseif ($b) { echo 2; } else { echo 3; }",
            3,
        ),
        // `else if` written as two words must score identically to `elseif`
        ("if ($a) { echo 1; } else if ($b) { echo 2; }", 2),
        ("if ($a) { echo 1; } elseif ($b) { echo 2; }", 2),
    ]);
}

#[test]
fn switch_and_match_cost_one_regardless_of_arm_count() {
    assert_scores(&[
        (
            "switch ($a) { case 1: echo 1; break; case 2: echo 2; break; default: echo 3; }",
            1,
        ),
        ("$r = match ($a) { 1 => 'a', 2 => 'b', default => 'c' };", 1),
    ]);
}

#[test]
fn try_is_free_and_each_catch_costs_one() {
    assert_scores(&[
        ("try { foo(); } catch (Exception $e) { echo 1; }", 1),
        (
            "try { foo(); } catch (A $e) { echo 1; } catch (B $e) { echo 2; }",
            2,
        ),
        // try does not raise nesting: the catch stays at +1
        (
            "try { foo(); } catch (Exception $e) { if ($a) { echo 1; } }",
            3,
        ),
    ]);
}

#[test]
fn multi_level_jumps_cost_one_and_plain_jumps_are_free() {
    assert_scores(&[
        ("foreach ($a as $x) { break; }", 1),
        // foreach +1, foreach +2, break 2 +1
        ("foreach ($a as $x) { foreach ($x as $y) { break 2; } }", 4),
        ("goto end;", 1),
    ]);
}

#[test]
fn closures_raise_nesting_without_scoring() {
    assert_scores(&[
        // closure itself +0, the if inside it +1+1
        ("$f = function () { if ($a) { echo 1; } };", 2),
        ("$f = fn () => $a ? 1 : 2;", 2),
    ]);
}

#[test]
fn direct_recursion_costs_one() {
    assert_scores(&[
        ("return target();", 1),
        ("return $this->target();", 1),
        // a call on another receiver is not recursion
        ("return $other->target();", 0),
        ("return unrelated();", 0),
    ]);
}

#[test]
fn a_linear_function_scores_zero() {
    assert_scores(&[("$a = 1; $b = 2; return $a + $b;", 0)]);
}
