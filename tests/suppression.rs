use phpcognit::complexity::Suppression;
use phpcognit::Finding;

fn analyze(source: &str) -> Vec<Finding> {
    let mut parser = phpcognit::parser().expect("PHP grammar should load");
    phpcognit::analyze_source(&mut parser, source)
}

fn only(source: &str) -> Finding {
    let findings = analyze(source);
    assert_eq!(findings.len(), 1, "fixture should yield one finding");
    findings.into_iter().next().expect("checked above")
}

const TANGLED_BODY: &str = "if ($a) { if ($b) { echo 1; } }";

fn with_leading(leading: &str) -> String {
    format!("<?php\n{leading}\nfunction target($a, $b) {{\n    {TANGLED_BODY}\n}}\n")
}

#[test]
fn a_reasoned_marker_above_the_function_suppresses() {
    let finding = only(&with_leading(
        "// phpcognit-ignore: hand-rolled parser, splitting hides the grammar",
    ));

    assert_eq!(
        finding.suppression,
        Suppression::Reasoned("hand-rolled parser, splitting hides the grammar".into())
    );
}

#[test]
fn a_marker_inside_a_docblock_suppresses() {
    let finding = only(&with_leading(
        "/**\n * Parses the wire format.\n * phpcognit-ignore: the grammar is flat but long\n */",
    ));

    assert_eq!(
        finding.suppression,
        Suppression::Reasoned("the grammar is flat but long".into())
    );
}

#[test]
fn a_single_line_docblock_marker_does_not_swallow_the_comment_terminator() {
    let finding = only(&with_leading("/** phpcognit-ignore: irreducible */"));

    assert_eq!(
        finding.suppression,
        Suppression::Reasoned("irreducible".into())
    );
}

#[test]
fn a_bare_marker_is_refused() {
    assert_eq!(
        only(&with_leading("// phpcognit-ignore")).suppression,
        Suppression::MissingReason
    );
}

#[test]
fn a_marker_with_an_empty_reason_is_refused() {
    assert_eq!(
        only(&with_leading("// phpcognit-ignore:")).suppression,
        Suppression::MissingReason
    );
    assert_eq!(
        only(&with_leading("// phpcognit-ignore:    ")).suppression,
        Suppression::MissingReason
    );
}

#[test]
fn an_unmarked_function_is_not_suppressed() {
    assert_eq!(
        only(&with_leading("// just an ordinary comment")).suppression,
        Suppression::None
    );
    assert_eq!(
        only("<?php\nfunction target($a, $b) { if ($a) { if ($b) { echo 1; } } }\n").suppression,
        Suppression::None
    );
}

#[test]
fn a_marker_survives_attributes_between_it_and_the_declaration() {
    let source = r"<?php
class Example {
    // phpcognit-ignore: dispatch table, flat by nature
    #[SomeAttribute]
    public function target($a, $b) {
        if ($a) { if ($b) { echo 1; } }
    }
}
";

    let finding = only(source);
    assert_eq!(
        finding.suppression,
        Suppression::Reasoned("dispatch table, flat by nature".into())
    );
}

/// A marker must not leak onto whatever is declared after the function it
/// annotates, or one comment would silence an entire file.
#[test]
fn a_marker_applies_only_to_the_declaration_it_precedes() {
    let findings = analyze(
        r"<?php
class Example {
    // phpcognit-ignore: only this one
    public function suppressed($a, $b) {
        if ($a) { if ($b) { echo 1; } }
    }

    public function untouched($a, $b) {
        if ($a) { if ($b) { echo 1; } }
    }
}
",
    );

    let suppressed = findings
        .iter()
        .find(|finding| finding.name == "suppressed")
        .expect("suppressed method");
    let untouched = findings
        .iter()
        .find(|finding| finding.name == "untouched")
        .expect("untouched method");

    assert_eq!(
        suppressed.suppression,
        Suppression::Reasoned("only this one".into())
    );
    assert_eq!(untouched.suppression, Suppression::None);
}

#[test]
fn suppression_does_not_change_the_score() {
    let plain = only(&with_leading("// ordinary"));
    let marked = only(&with_leading("// phpcognit-ignore: deliberate"));

    assert_eq!(plain.score, marked.score);
    assert!(marked.score > 0, "fixture should score above zero");
}
