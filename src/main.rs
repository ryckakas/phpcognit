use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser as ClapParser, ValueEnum};
use ignore::WalkBuilder;
use serde::Serialize;

use phpcognit::complexity::{Suppression, SUPPRESSION_MARKER};
use phpcognit::{Baseline, Finding};

#[derive(ClapParser)]
#[command(
    name = "phpcognit",
    version,
    about = "Cognitive complexity linter for PHP"
)]
struct Args {
    /// Files or directories to scan
    #[arg(default_value = ".")]
    paths: Vec<PathBuf>,

    /// Complexity threshold; functions above it fail the run
    #[arg(long, default_value_t = 15)]
    over: u32,

    /// Print every function, not just those above the threshold
    #[arg(long)]
    all: bool,

    /// Output format; `json` is the machine-readable form editors consume
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,

    /// Record current findings as accepted, so only later regressions fail
    #[arg(long)]
    write_baseline: bool,

    /// Baseline file; used when it exists, ignored when it does not
    #[arg(long, default_value = ".phpcognit-baseline.json")]
    baseline: PathBuf,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum Format {
    Text,
    Json,
}

/// `complexity::Finding` stays free of paths and serialisation so the scorer
/// remains embeddable without dragging serde in.
#[derive(Serialize)]
struct Report {
    threshold: u32,
    breaches: usize,
    findings: Vec<ReportedFinding>,
}

#[derive(Serialize)]
struct ReportedFinding {
    path: String,
    line: usize,
    name: String,
    score: u32,
}

fn main() -> ExitCode {
    let args = Args::parse();

    let mut parser = match phpcognit::parser() {
        Ok(parser) => parser,
        Err(error) => {
            eprintln!("failed to load the PHP grammar: {error}");
            return ExitCode::FAILURE;
        }
    };

    let mut findings: Vec<(PathBuf, Finding)> = Vec::new();
    for path in &args.paths {
        collect(path, &mut parser, &mut findings);
    }

    findings.sort_by(|left, right| {
        right
            .1
            .score
            .cmp(&left.1.score)
            .then_with(|| left.0.cmp(&right.0))
    });

    warn_about_unreasoned_suppressions(&findings, &args);

    let over_threshold: Vec<(PathBuf, Finding)> = findings
        .iter()
        .filter(|(_, finding)| {
            finding.score > args.over && !matches!(finding.suppression, Suppression::Reasoned(_))
        })
        .cloned()
        .collect();

    if args.write_baseline {
        return write_baseline(&over_threshold, &args);
    }

    let baseline = match load_baseline(&args.baseline) {
        Ok(baseline) => baseline,
        Err(error) => {
            eprintln!("{}: {error}", args.baseline.display());
            return ExitCode::FAILURE;
        }
    };

    let breaches = match &baseline {
        Some(baseline) => over_threshold
            .iter()
            .filter(|(path, finding)| baseline.is_regression(path, finding))
            .count(),
        None => over_threshold.len(),
    };

    match args.format {
        Format::Text => print_text(&findings, &args, baseline.as_ref()),
        Format::Json => print_json(&findings, &args, breaches, baseline.as_ref()),
    }

    if breaches == 0 {
        return ExitCode::SUCCESS;
    }

    if args.format == Format::Text {
        let accepted = over_threshold.len() - breaches;
        eprintln!(
            "\n{breaches} function(s) over the threshold of {}{}",
            args.over,
            if accepted > 0 {
                format!(", {accepted} accepted by the baseline")
            } else {
                String::new()
            }
        );
    }

    ExitCode::FAILURE
}

fn load_baseline(path: &Path) -> io::Result<Option<Baseline>> {
    if path.exists() {
        Baseline::load(path).map(Some)
    } else {
        Ok(None)
    }
}

fn write_baseline(over_threshold: &[(PathBuf, Finding)], args: &Args) -> ExitCode {
    let baseline = Baseline::from_findings(over_threshold);

    if let Err(error) = baseline.save(&args.baseline) {
        eprintln!("{}: {error}", args.baseline.display());
        return ExitCode::FAILURE;
    }

    println!(
        "recorded {} finding(s) above {} in {}",
        baseline.len(),
        args.over,
        args.baseline.display()
    );

    ExitCode::SUCCESS
}

fn is_reported(baseline: Option<&Baseline>, path: &Path, finding: &Finding, args: &Args) -> bool {
    if finding.score <= args.over || matches!(finding.suppression, Suppression::Reasoned(_)) {
        return false;
    }

    baseline.is_none_or(|baseline| baseline.is_regression(path, finding))
}

/// A marker without a reason is refused rather than obeyed, so it has to say so
/// out loud — otherwise the author would believe the finding was silenced.
fn warn_about_unreasoned_suppressions(findings: &[(PathBuf, Finding)], args: &Args) {
    for (path, finding) in findings {
        if finding.score > args.over && finding.suppression == Suppression::MissingReason {
            eprintln!(
                "{}:{}: `{SUPPRESSION_MARKER}` needs a reason, e.g. `// {SUPPRESSION_MARKER}: why this has to stay complex` — ignoring it and reporting {}",
                path.display(),
                finding.line,
                finding.qualified_name()
            );
        }
    }
}

fn print_text(findings: &[(PathBuf, Finding)], args: &Args, baseline: Option<&Baseline>) {
    for (path, finding) in findings {
        if args.all || is_reported(baseline, path, finding, args) {
            println!(
                "{:>4}  {}:{}  {}",
                finding.score,
                path.display(),
                finding.line,
                finding.qualified_name()
            );
        }
    }
}

fn print_json(
    findings: &[(PathBuf, Finding)],
    args: &Args,
    breaches: usize,
    baseline: Option<&Baseline>,
) {
    let reported: Vec<ReportedFinding> = findings
        .iter()
        .filter(|(path, finding)| args.all || is_reported(baseline, path, finding, args))
        .map(|(path, finding)| ReportedFinding {
            path: path.display().to_string(),
            line: finding.line,
            name: finding.qualified_name(),
            score: finding.score,
        })
        .collect();

    let report = Report {
        threshold: args.over,
        breaches,
        findings: reported,
    };

    match serde_json::to_string_pretty(&report) {
        Ok(json) => println!("{json}"),
        Err(error) => eprintln!("failed to serialise the report: {error}"),
    }
}

fn collect(root: &Path, parser: &mut tree_sitter::Parser, findings: &mut Vec<(PathBuf, Finding)>) {
    for entry in WalkBuilder::new(root).build() {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                eprintln!("{error}");
                continue;
            }
        };

        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }

        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("php") {
            continue;
        }

        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("{}: {error}", path.display());
                continue;
            }
        };

        for finding in phpcognit::analyze_source(parser, &source) {
            findings.push((path.to_path_buf(), finding));
        }
    }
}
