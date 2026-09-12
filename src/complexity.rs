//! Cognitive complexity scoring, per the `SonarSource` specification.
//!
//! Three rules drive everything here:
//!   1. Shorthand structures that don't break reading flow score nothing (`??`).
//!   2. +1 for each break in the linear flow of the code.
//!   3. +nesting for a flow-breaker that sits inside other flow-breakers.
//!
//! Deliberately parser-adjacent but I/O-free: it takes a parsed tree and the
//! source bytes, and returns scores. No filesystem, no config, no reporting.

use tree_sitter::Node;

use crate::kinds;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub name: String,
    pub line: usize,
    pub score: u32,
}

#[must_use]
pub fn analyze(tree: &tree_sitter::Tree, src: &[u8]) -> Vec<Finding> {
    let mut findings = Vec::new();
    collect_units(tree.root_node(), src, &mut findings);
    findings
}

fn collect_units(node: Node<'_>, src: &[u8], findings: &mut Vec<Finding>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if kinds::is_scoring_unit(child.kind()) {
            findings.push(score_unit(child, src));
        } else {
            collect_units(child, src, findings);
        }
    }
}

fn score_unit(node: Node<'_>, src: &[u8]) -> Finding {
    let name = unit_name(node, src);
    let mut score = 0;

    if let Some(body) = node.child_by_field_name("body") {
        walk(body, 0, &name, src, &mut score);
    }

    Finding {
        name,
        line: node.start_position().row + 1,
        score,
    }
}

fn unit_name(node: Node<'_>, src: &[u8]) -> String {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(src).ok())
        .unwrap_or("<anonymous>")
        .to_string()
}

fn walk(node: Node<'_>, nesting: u32, unit: &str, src: &[u8], score: &mut u32) {
    let kind = node.kind();

    // A closure raises nesting for everything inside it but costs nothing itself.
    if kinds::is_nested_function(kind) {
        walk_children(node, nesting + 1, unit, src, score);
        return;
    }

    match kind {
        kinds::IF_STATEMENT => {
            walk_if(node, nesting, false, unit, src, score);
            return;
        }

        // Every one of these both scores and nests.
        kinds::CONDITIONAL_EXPRESSION
        | kinds::SWITCH_STATEMENT
        | kinds::MATCH_EXPRESSION
        | kinds::FOR_STATEMENT
        | kinds::FOREACH_STATEMENT
        | kinds::WHILE_STATEMENT
        | kinds::DO_STATEMENT
        | kinds::CATCH_CLAUSE => {
            *score += 1 + nesting;
            walk_control(node, nesting, unit, src, score);
            return;
        }

        // A jump out of more than one enclosing structure is PHP's analogue of
        // the spec's labelled break. Plain `break;` reads linearly, so it's free.
        kinds::BREAK_STATEMENT | kinds::CONTINUE_STATEMENT => {
            if is_multi_level_jump(node, src) {
                *score += 1;
            }
            return;
        }

        kinds::GOTO_STATEMENT => {
            *score += 1;
            return;
        }

        kinds::BINARY_EXPRESSION => {
            if logical_operator(node, src).is_some() {
                walk_logical_sequence(node, nesting, unit, src, score);
                return;
            }
        }

        kinds::FUNCTION_CALL | kinds::MEMBER_CALL | kinds::SCOPED_CALL
            if is_recursive_call(node, unit, src) =>
        {
            *score += 1;
        }

        _ => {}
    }

    walk_children(node, nesting, unit, src, score);
}

/// `flat` marks an `if` that is really the tail of an `else if`, which the spec
/// scores as a single flat +1 so that long chains aren't punished for depth.
fn walk_if(node: Node<'_>, nesting: u32, flat: bool, unit: &str, src: &[u8], score: &mut u32) {
    *score += if flat { 1 } else { 1 + nesting };

    if let Some(condition) = node.child_by_field_name("condition") {
        walk(condition, nesting, unit, src, score);
    }
    if let Some(body) = node.child_by_field_name("body") {
        walk(body, nesting + 1, unit, src, score);
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            kinds::ELSE_IF_CLAUSE => {
                *score += 1;
                if let Some(condition) = child.child_by_field_name("condition") {
                    walk(condition, nesting, unit, src, score);
                }
                if let Some(body) = child.child_by_field_name("body") {
                    walk(body, nesting + 1, unit, src, score);
                }
            }
            kinds::ELSE_CLAUSE => walk_else(child, nesting, unit, src, score),
            _ => {}
        }
    }
}

/// `else if` written as two words parses as an `else_clause` wrapping an
/// `if_statement`. It must score the same as `elseif`, so the inner `if` takes
/// the flat increment and the `else` itself adds nothing.
fn walk_else(node: Node<'_>, nesting: u32, unit: &str, src: &[u8], score: &mut u32) {
    let body = node
        .child_by_field_name("body")
        .or_else(|| node.named_child(0));

    match body {
        Some(inner) if inner.kind() == kinds::IF_STATEMENT => {
            walk_if(inner, nesting, true, unit, src, score);
        }
        Some(inner) => {
            *score += 1;
            walk(inner, nesting + 1, unit, src, score);
        }
        None => *score += 1,
    }
}

/// Branch bodies nest; the controlling condition does not.
fn walk_control(node: Node<'_>, nesting: u32, unit: &str, src: &[u8], score: &mut u32) {
    let condition_id = node.child_by_field_name("condition").map(|c| c.id());

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        let child_nesting = if Some(child.id()) == condition_id {
            nesting
        } else {
            nesting + 1
        };
        walk(child, child_nesting, unit, src, score);
    }
}

fn walk_children(node: Node<'_>, nesting: u32, unit: &str, src: &[u8], score: &mut u32) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk(child, nesting, unit, src, score);
    }
}

/// A run of like operators costs +1 however long it is; the cost is in the
/// switching. `$a && $b && $c` is +1, `$a && $b || $c` is +2.
fn walk_logical_sequence(node: Node<'_>, nesting: u32, unit: &str, src: &[u8], score: &mut u32) {
    let mut operators = Vec::new();
    let mut operands = Vec::new();
    flatten_logical(node, src, &mut operators, &mut operands);

    *score += count_runs(&operators);

    for operand in operands {
        walk(operand, nesting, unit, src, score);
    }
}

/// Flattens the left-nested operator tree into source order. Flattening stops
/// at anything that isn't itself a logical `binary_expression`, so a
/// parenthesised sub-expression starts a fresh sequence — which is what makes
/// `$a && ($b || $c)` score 2 rather than 1.
fn flatten_logical<'t>(
    node: Node<'t>,
    src: &[u8],
    operators: &mut Vec<&'static str>,
    operands: &mut Vec<Node<'t>>,
) {
    if let Some(left) = node.child_by_field_name("left") {
        if logical_operator(left, src).is_some() {
            flatten_logical(left, src, operators, operands);
        } else {
            operands.push(left);
        }
    }

    if let Some(operator) = logical_operator(node, src) {
        operators.push(operator);
    }

    if let Some(right) = node.child_by_field_name("right") {
        if logical_operator(right, src).is_some() {
            flatten_logical(right, src, operators, operands);
        } else {
            operands.push(right);
        }
    }
}

fn count_runs(operators: &[&str]) -> u32 {
    let mut runs = 0;
    let mut previous: Option<&str> = None;

    for &operator in operators {
        if previous != Some(operator) {
            runs += 1;
            previous = Some(operator);
        }
    }

    runs
}

/// `and`/`or` are the same logical operators as `&&`/`||` with different
/// precedence, so they normalise together for run-counting purposes.
fn logical_operator(node: Node<'_>, src: &[u8]) -> Option<&'static str> {
    if node.kind() != kinds::BINARY_EXPRESSION {
        return None;
    }

    let text = node
        .child_by_field_name("operator")?
        .utf8_text(src)
        .ok()?
        .to_ascii_lowercase();

    match text.as_str() {
        "&&" | "and" => Some("&&"),
        "||" | "or" => Some("||"),
        "xor" => Some("xor"),
        _ => None,
    }
}

fn is_multi_level_jump(node: Node<'_>, src: &[u8]) -> bool {
    node.named_child(0)
        .filter(|level| level.kind() == kinds::INTEGER)
        .and_then(|level| level.utf8_text(src).ok())
        .and_then(|text| text.parse::<u32>().ok())
        .is_some_and(|level| level > 1)
}

/// Only direct syntactic self-reference is detectable without symbol
/// resolution; dynamic dispatch through a variable is out of reach and is
/// documented as such rather than guessed at.
fn is_recursive_call(node: Node<'_>, unit: &str, src: &[u8]) -> bool {
    let callee = match node.kind() {
        kinds::FUNCTION_CALL => node.child_by_field_name("function"),
        kinds::MEMBER_CALL | kinds::SCOPED_CALL => {
            if !is_self_reference(node, src) {
                return false;
            }
            node.child_by_field_name("name")
        }
        _ => None,
    };

    callee
        .and_then(|c| c.utf8_text(src).ok())
        .is_some_and(|text| text == unit)
}

fn is_self_reference(node: Node<'_>, src: &[u8]) -> bool {
    let receiver = node
        .child_by_field_name("object")
        .or_else(|| node.child_by_field_name("scope"));

    receiver
        .and_then(|r| r.utf8_text(src).ok())
        .is_some_and(|text| matches!(text, "$this" | "self" | "static"))
}
