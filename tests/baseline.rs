use std::path::{Path, PathBuf};

use phpcognit::{Baseline, Finding};

fn analyze(source: &str) -> Vec<(PathBuf, Finding)> {
    let mut parser = phpcognit::parser().expect("PHP grammar should load");

    phpcognit::analyze_source(&mut parser, source)
        .into_iter()
        .map(|finding| (PathBuf::from("src/Example.php"), finding))
        .collect()
}

fn find<'a>(findings: &'a [(PathBuf, Finding)], qualified: &str) -> &'a Finding {
    findings
        .iter()
        .map(|(_, finding)| finding)
        .find(|finding| finding.qualified_name() == qualified)
        .unwrap_or_else(|| {
            let present: Vec<_> = findings
                .iter()
                .map(|(_, finding)| finding.qualified_name())
                .collect();
            panic!("no finding {qualified}; found {present:?}")
        })
}

const NESTED: &str = r"<?php
class Example {
    public function tangled($a, $b) {
        if ($a) { if ($b) { echo 1; } }
    }
}
";

#[test]
fn a_recorded_finding_is_accepted() {
    let findings = analyze(NESTED);
    let baseline = Baseline::from_findings(&findings);

    let path = Path::new("src/Example.php");
    assert!(!baseline.is_regression(path, find(&findings, "Example::tangled")));
}

#[test]
fn an_unrecorded_finding_is_a_regression() {
    let baseline = Baseline::default();

    let findings = analyze(NESTED);
    let path = Path::new("src/Example.php");

    assert!(baseline.is_regression(path, find(&findings, "Example::tangled")));
    assert!(baseline.is_empty());
}

#[test]
fn a_worse_score_is_a_regression_but_an_equal_or_better_one_is_not() {
    let findings = analyze(NESTED);
    let baseline = Baseline::from_findings(&findings);
    let path = Path::new("src/Example.php");

    let recorded = find(&findings, "Example::tangled").clone();
    assert!(recorded.score > 0, "fixture should score above zero");

    let worse = Finding {
        score: recorded.score + 1,
        ..recorded.clone()
    };
    let better = Finding {
        score: recorded.score - 1,
        ..recorded.clone()
    };

    assert!(baseline.is_regression(path, &worse));
    assert!(!baseline.is_regression(path, &recorded));
    assert!(!baseline.is_regression(path, &better));
}

/// The reason the baseline keys on function identity rather than on the file:
/// a grandfathered file must not become a place where new complexity hides.
#[test]
fn a_new_function_in_a_recorded_file_is_still_a_regression() {
    let baseline = Baseline::from_findings(&analyze(NESTED));

    let grown = analyze(
        r"<?php
class Example {
    public function tangled($a, $b) {
        if ($a) { if ($b) { echo 1; } }
    }
    public function addedLater($a, $b) {
        if ($a) { if ($b) { echo 1; } }
    }
}
",
    );

    let path = Path::new("src/Example.php");
    assert!(!baseline.is_regression(path, find(&grown, "Example::tangled")));
    assert!(baseline.is_regression(path, find(&grown, "Example::addedLater")));
}

#[test]
fn same_method_name_in_two_classes_stays_distinct() {
    let findings = analyze(
        r"<?php
class First {
    public function process($a, $b) {
        if ($a) { if ($b) { echo 1; } }
    }
}
class Second {
    public function process($a, $b) {
        if ($a) { if ($b) { if ($a) { echo 1; } } }
    }
}
",
    );

    let first = find(&findings, "First::process");
    let second = find(&findings, "Second::process");
    assert_ne!(first.score, second.score, "fixture should differ in score");

    let only_first = Baseline::from_findings(&[(
        PathBuf::from("src/Example.php"),
        find(&findings, "First::process").clone(),
    )]);

    let path = Path::new("src/Example.php");
    assert!(!only_first.is_regression(path, first));
    assert!(only_first.is_regression(path, second));
}

#[test]
fn a_free_function_has_no_class_qualifier() {
    let findings = analyze(
        r"<?php
function loose($a, $b) {
    if ($a) { if ($b) { echo 1; } }
}
",
    );

    let finding = find(&findings, "loose");
    assert_eq!(finding.class, None);
    assert_eq!(finding.qualified_name(), "loose");
}

#[test]
fn a_round_trip_through_disk_preserves_decisions() {
    let findings = analyze(NESTED);
    let baseline = Baseline::from_findings(&findings);

    let file = std::env::temp_dir().join(format!("phpcognit-baseline-{}.json", std::process::id()));
    baseline.save(&file).expect("baseline should save");

    let reloaded = Baseline::load(&file).expect("baseline should load");
    std::fs::remove_file(&file).ok();

    let path = Path::new("src/Example.php");
    assert_eq!(reloaded.len(), baseline.len());
    assert!(!reloaded.is_regression(path, find(&findings, "Example::tangled")));
}
