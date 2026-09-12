use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser as ClapParser, ValueEnum};
use ignore::WalkBuilder;
use serde::Serialize;

use phpcognit::Finding;

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
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum Format {
    Text,
    Json,
}

/// The CLI's own view of a finding. `complexity::Finding` stays free of paths
/// and serialisation so the scorer remains embeddable without dragging serde in.
#[derive(Serialize)]
struct Report<'a> {
    threshold: u32,
    breaches: usize,
    findings: Vec<ReportedFinding<'a>>,
}

#[derive(Serialize)]
struct ReportedFinding<'a> {
    path: String,
    line: usize,
    name: &'a str,
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

    let breaches = findings
        .iter()
        .filter(|(_, finding)| finding.score > args.over)
        .count();

    match args.format {
        Format::Text => print_text(&findings, &args),
        Format::Json => print_json(&findings, &args, breaches),
    }

    if breaches == 0 {
        return ExitCode::SUCCESS;
    }

    if args.format == Format::Text {
        eprintln!(
            "\n{breaches} function(s) over the threshold of {}",
            args.over
        );
    }

    ExitCode::FAILURE
}

fn print_text(findings: &[(PathBuf, Finding)], args: &Args) {
    for (path, finding) in findings {
        if args.all || finding.score > args.over {
            println!(
                "{:>4}  {}:{}  {}",
                finding.score,
                path.display(),
                finding.line,
                finding.name
            );
        }
    }
}

fn print_json(findings: &[(PathBuf, Finding)], args: &Args, breaches: usize) {
    let reported: Vec<ReportedFinding<'_>> = findings
        .iter()
        .filter(|(_, finding)| args.all || finding.score > args.over)
        .map(|(path, finding)| ReportedFinding {
            path: path.display().to_string(),
            line: finding.line,
            name: &finding.name,
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
