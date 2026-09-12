//! Guards the node kind strings in `kinds.rs` against a grammar upgrade
//! silently renaming something. Without this, a renamed node would not fail to
//! compile — it would just stop matching, and every score would quietly drop.

use std::collections::HashSet;

use phpcognit::kinds;
use tree_sitter::Node;

const FIXTURE: &str = r"<?php
function everything($a, $b) {
    if ($a) { echo 1; } elseif ($b) { echo 2; } else { echo 3; }
    $ternary = $a ? 1 : 2;
    switch ($a) { case 1: echo 1; break; default: echo 2; }
    $matched = match ($a) { 1 => 'a', default => 'b' };
    for ($i = 0; $i < 3; $i++) { echo $i; }
    foreach ($a as $x) { echo $x; }
    while ($a) { break 2; }
    do { echo 1; } while ($a);
    try { everything(1, 2); } catch (Exception $e) { continue 2; }
    if ($a && $b) { echo 1; }
    $closure = function () { return 1; };
    $arrow = fn () => 1;
    $this->everything(1, 2);
    self::everything(1, 2);
    goto finish;
    finish:
}

class Sample {
    public function method() { return 1; }
}
";

fn fixture_kinds() -> HashSet<String> {
    let mut parser = phpcognit::parser().expect("PHP grammar should load");
    let tree = parser.parse(FIXTURE, None).expect("fixture should parse");

    let mut found = HashSet::new();
    collect_kinds(tree.root_node(), &mut found);
    found
}

fn collect_kinds(node: Node<'_>, found: &mut HashSet<String>) {
    found.insert(node.kind().to_string());

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_kinds(child, found);
    }
}

#[test]
fn every_declared_kind_still_exists_in_the_grammar() {
    let found = fixture_kinds();

    let required = [
        kinds::IF_STATEMENT,
        kinds::ELSE_IF_CLAUSE,
        kinds::ELSE_CLAUSE,
        kinds::CONDITIONAL_EXPRESSION,
        kinds::SWITCH_STATEMENT,
        kinds::MATCH_EXPRESSION,
        kinds::FOR_STATEMENT,
        kinds::FOREACH_STATEMENT,
        kinds::WHILE_STATEMENT,
        kinds::DO_STATEMENT,
        kinds::CATCH_CLAUSE,
        kinds::GOTO_STATEMENT,
        kinds::BREAK_STATEMENT,
        kinds::CONTINUE_STATEMENT,
        kinds::BINARY_EXPRESSION,
        kinds::INTEGER,
        kinds::FUNCTION_DEFINITION,
        kinds::METHOD_DECLARATION,
        kinds::ARROW_FUNCTION,
        kinds::FUNCTION_CALL,
        kinds::MEMBER_CALL,
        kinds::SCOPED_CALL,
    ];

    let missing: Vec<_> = required
        .iter()
        .filter(|kind| !found.contains(**kind))
        .collect();

    assert!(
        missing.is_empty(),
        "node kinds absent from the grammar (renamed upstream?): {missing:?}"
    );
}

#[test]
fn one_of_the_anonymous_function_spellings_is_live() {
    let found = fixture_kinds();

    assert!(
        found.contains(kinds::ANONYMOUS_FUNCTION)
            || found.contains(kinds::ANONYMOUS_FUNCTION_CREATION),
        "neither anonymous function spelling matched; grammar now uses something else"
    );
}

#[test]
fn the_fixture_itself_is_valid_php() {
    let mut parser = phpcognit::parser().expect("PHP grammar should load");
    let tree = parser.parse(FIXTURE, None).expect("fixture should parse");

    assert!(
        !tree.root_node().has_error(),
        "the grammar fixture no longer parses cleanly; fix the fixture before trusting the kind checks"
    );
}
