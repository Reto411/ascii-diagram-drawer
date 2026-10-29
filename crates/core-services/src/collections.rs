//! Collection discovery at startup (S-016, FR3): scan a directory of `.adc` files and
//! load each into the registry. Invalid files are **skipped with a warning**, never an
//! error — startup always succeeds (RS family). The GUI shows the warnings; it does
//! not do filesystem discovery itself (ADR 009: catalog data crosses via the seam,
//! discovery is a composition-root concern).

use std::path::Path;

use shape_registry::{ShapeRegistry, parse_collection};

/// A collection file that was skipped at startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionLoadWarning {
    /// The offending file name (or the directory itself when it cannot be read).
    pub file: String,
    /// Why it was skipped — parse errors carry line/column; IO errors carry the cause.
    pub reason: String,
}

impl CollectionLoadWarning {
    /// One-line form for GUI notices: `file.adc: <reason>`.
    #[must_use]
    pub fn to_notice(&self) -> String {
        format!("{}: {}", self.file, self.reason)
    }
}

/// Loads every `*.adc` file in `dir` (sorted by file name, deterministic order) into
/// `registry`, returning one warning per skipped file.
///
/// A missing/unreadable directory is a single warning — not an error: startup always
/// succeeds with at least the built-ins (S-016 AC2).
#[must_use]
pub fn load_from_dir(registry: &mut ShapeRegistry, dir: &Path) -> Vec<CollectionLoadWarning> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            return vec![CollectionLoadWarning {
                file: dir.display().to_string(),
                reason: e.to_string(),
            }];
        }
    };

    let mut files: Vec<_> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "adc"))
        .collect();
    files.sort();

    let mut warnings = Vec::new();
    for path in files {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        match std::fs::read_to_string(&path) {
            Ok(text) => match parse_collection(
                path.file_stem().and_then(|s| s.to_str()).unwrap_or(""),
                &text,
            ) {
                Ok(file) => registry.add_collection(file.collection),
                Err(e) => warnings.push(CollectionLoadWarning {
                    file: name,
                    reason: e.to_string(),
                }),
            },
            Err(e) => warnings.push(CollectionLoadWarning {
                file: name,
                reason: e.to_string(),
            }),
        }
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal valid collection file (C8 grammar).
    fn valid_adc() -> String {
        "adc 1.0\ntemplate star 3 x 3\n###\n#.#\n###\nanchor n at 1 0\n".to_string()
    }

    /// AC2 — a directory of valid files loads every collection, in file-name order.
    #[test]
    fn valid_files_load_in_sorted_order() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("b.adc"), valid_adc()).unwrap();
        std::fs::write(dir.path().join("a.adc"), valid_adc()).unwrap();
        // Non-collection files are ignored.
        std::fs::write(dir.path().join("notes.txt"), "not a collection").unwrap();

        let mut registry = ShapeRegistry::builtin();
        let warnings = load_from_dir(&mut registry, dir.path());
        assert!(warnings.is_empty(), "{warnings:?}");
        let names: Vec<&str> = registry.collections().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b"]);
    }

    /// AC2 — an invalid file is skipped with a line/column warning; the others load.
    #[test]
    fn invalid_file_is_skipped_with_a_warning_and_startup_succeeds() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("good.adc"), valid_adc()).unwrap();
        std::fs::write(
            dir.path().join("bad.adc"),
            // A grid row longer than the declared width → parse error (short rows are
            // fine — right-padded, documented parser leniency).
            "adc 1.0\ntemplate x 3 x 2\nabcd\ncd\n",
        )
        .unwrap();

        let mut registry = ShapeRegistry::builtin();
        let warnings = load_from_dir(&mut registry, dir.path());

        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].file, "bad.adc");
        assert!(
            warnings[0].reason.starts_with("line "),
            "parse errors carry line/column: {:?}",
            warnings[0].reason
        );
        let names: Vec<&str> = registry.collections().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["good"]);
    }

    /// AC2 — an empty directory loads nothing and warns nothing.
    #[test]
    fn empty_dir_is_silent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut registry = ShapeRegistry::builtin();
        assert!(load_from_dir(&mut registry, dir.path()).is_empty());
        assert!(registry.collections().next().is_none());
    }

    /// AC2 — a missing directory is one warning, not an error; built-ins survive.
    #[test]
    fn missing_dir_is_a_warning_not_an_error() {
        let mut registry = ShapeRegistry::builtin();
        let warnings = load_from_dir(&mut registry, Path::new("/nonexistent-collections-dir"));
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].file, "/nonexistent-collections-dir");
        assert!(registry.builtins().next().is_some());
    }

    /// The notice form is what the GUI status bar prints.
    #[test]
    fn notice_is_file_and_reason() {
        let w = CollectionLoadWarning {
            file: "x.adc".into(),
            reason: "line 2, column 1: boom".into(),
        };
        assert_eq!(w.to_notice(), "x.adc: line 2, column 1: boom");
    }
}
