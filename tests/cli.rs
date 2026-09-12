//! Exit-code contract. A gate that cannot read its input must never report
//! success, so these guard the silent-pass failures rather than the happy path.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("phpcognit-cli-{}-{name}", std::process::id()));
    fs::remove_dir_all(&dir).ok();
    fs::create_dir_all(&dir).expect("scratch directory");
    dir
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_phpcognit"))
        .args(args)
        .output()
        .expect("binary should run")
}

const TANGLED: &str = r"<?php
class Tangled {
    public function knot($a, $b, $c, $d, $e, $f) {
        if ($a) { if ($b) { if ($c) { if ($d) { if ($e) { if ($f) { echo 1; } } } } } }
    }
}
";

const LINEAR: &str = r"<?php
class Linear {
    public function plain($a) {
        return $a + 1;
    }
}
";

#[test]
fn a_path_that_does_not_exist_fails() {
    let output = run(&["--over", "15", "definitely/not/here"]);

    assert!(
        !output.status.success(),
        "a missing path must not report a clean run"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("refusing to report a clean run"));
}

#[test]
fn a_directory_without_php_fails() {
    let dir = scratch("nophp");
    fs::write(dir.join("notes.md"), "nothing to see").expect("fixture");

    let output = run(&["--over", "15", dir.to_str().expect("utf8 path")]);

    assert!(
        !output.status.success(),
        "scanning zero files is never what was meant"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("no PHP files found"));
}

#[test]
fn a_clean_scan_succeeds_silently() {
    let dir = scratch("clean");
    fs::write(dir.join("Linear.php"), LINEAR).expect("fixture");

    let output = run(&["--over", "15", dir.to_str().expect("utf8 path")]);

    assert!(output.status.success());
    assert!(
        output.stdout.is_empty(),
        "success prints nothing, so stdout stays usable in a pipeline"
    );
}

#[test]
fn a_breach_fails_and_names_the_function() {
    let dir = scratch("breach");
    fs::write(dir.join("Tangled.php"), TANGLED).expect("fixture");

    let output = run(&["--over", "15", dir.to_str().expect("utf8 path")]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Tangled::knot"));
}

#[test]
fn a_baseline_accepts_what_it_recorded() {
    let dir = scratch("baseline");
    fs::write(dir.join("Tangled.php"), TANGLED).expect("fixture");
    let baseline = dir.join("baseline.json");

    let path = dir.to_str().expect("utf8 path");
    let baseline_arg = baseline.to_str().expect("utf8 path");

    let written = run(&[
        "--over",
        "15",
        "--baseline",
        baseline_arg,
        "--write-baseline",
        path,
    ]);
    assert!(written.status.success());

    let checked = run(&["--over", "15", "--baseline", baseline_arg, path]);
    assert!(
        checked.status.success(),
        "a recorded finding should no longer fail the run"
    );
}

#[test]
fn a_reasoned_suppression_clears_the_gate() {
    let dir = scratch("suppressed");
    fs::write(
        dir.join("Tangled.php"),
        TANGLED.replace(
            "    public function knot",
            "    // phpcognit-ignore: deliberate\n    public function knot",
        ),
    )
    .expect("fixture");

    let output = run(&["--over", "15", dir.to_str().expect("utf8 path")]);

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn a_suppression_without_a_reason_still_fails() {
    let dir = scratch("unreasoned");
    fs::write(
        dir.join("Tangled.php"),
        TANGLED.replace(
            "    public function knot",
            "    // phpcognit-ignore\n    public function knot",
        ),
    )
    .expect("fixture");

    let output = run(&["--over", "15", dir.to_str().expect("utf8 path")]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("needs a reason"));
}
