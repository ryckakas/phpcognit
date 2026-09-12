//! Recorded scores for code that already exists, so an established codebase can
//! adopt a threshold without fixing everything first.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Finding;

const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct Baseline {
    version: u32,
    entries: BTreeMap<String, BTreeMap<String, u32>>,
}

impl Default for Baseline {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            entries: BTreeMap::new(),
        }
    }
}

impl Baseline {
    #[must_use]
    pub fn from_findings(findings: &[(PathBuf, Finding)]) -> Self {
        let mut entries: BTreeMap<String, BTreeMap<String, u32>> = BTreeMap::new();

        for (path, finding) in findings {
            entries
                .entry(normalize(path))
                .or_default()
                .insert(finding.qualified_name(), finding.score);
        }

        Self {
            version: FORMAT_VERSION,
            entries,
        }
    }

    /// # Errors
    ///
    /// Fails if the file cannot be read, or holds a format this build does not
    /// understand.
    pub fn load(path: &Path) -> io::Result<Self> {
        let text = fs::read_to_string(path)?;
        let baseline: Self = serde_json::from_str(&text)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

        if baseline.version != FORMAT_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "baseline format version {} is not supported (expected {FORMAT_VERSION}); regenerate with --write-baseline",
                    baseline.version
                ),
            ));
        }

        Ok(baseline)
    }

    /// # Errors
    ///
    /// Fails if the file cannot be written.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let mut text = serde_json::to_string_pretty(self)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        text.push('\n');
        fs::write(path, text)
    }

    #[must_use]
    pub fn is_regression(&self, path: &Path, finding: &Finding) -> bool {
        self.entries
            .get(&normalize(path))
            .and_then(|functions| functions.get(&finding.qualified_name()))
            .is_none_or(|accepted| finding.score > *accepted)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.values().map(BTreeMap::len).sum()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Relative with forward slashes, so a baseline written on one machine still
/// matches when CI checks it out elsewhere, or on another platform.
fn normalize(path: &Path) -> String {
    let relative = std::env::current_dir()
        .ok()
        .and_then(|cwd| path.strip_prefix(cwd).ok().map(Path::to_path_buf))
        .unwrap_or_else(|| path.to_path_buf());

    relative.to_string_lossy().replace('\\', "/")
}
