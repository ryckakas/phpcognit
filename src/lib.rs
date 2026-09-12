//! Cognitive complexity for PHP.
//!
//! The scorer in [`complexity`] is the reusable half of this crate — it takes a
//! parsed tree and returns per-function scores, with no filesystem or CLI
//! coupling, so it can be embedded in other tooling.

pub mod baseline;
pub mod complexity;
pub mod kinds;

pub use baseline::Baseline;
pub use complexity::{analyze, Finding};

use tree_sitter::{LanguageError, Parser};

/// Builds a parser with the PHP grammar loaded.
///
/// # Errors
///
/// Fails if the linked grammar is incompatible with this tree-sitter version,
/// which in practice means a dependency bump moved one without the other.
pub fn parser() -> Result<Parser, LanguageError> {
    let mut parser = Parser::new();
    let language = tree_sitter_php::LANGUAGE_PHP;
    parser.set_language(&language.into())?;
    Ok(parser)
}

/// Returns no findings for input tree-sitter cannot parse at all. Partial
/// parses still score: tree-sitter recovers from syntax errors, and a file
/// mid-edit should not blank the report.
pub fn analyze_source(parser: &mut Parser, source: &str) -> Vec<Finding> {
    match parser.parse(source, None) {
        Some(tree) => analyze(&tree, source.as_bytes()),
        None => Vec::new(),
    }
}
