//! Golden file tests (S-004 AC2): every `.ad` file in `tests/golden/` is already in canonical
//! form — parsing it and re-serializing must be the identity (byte-for-byte, QS7).

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: golden files are trusted fixtures"
)]

use std::fs;
use std::path::{Path, PathBuf};

const GOLDEN_DIR: &str = "tests/golden";

/// All `.ad` golden files, sorted for deterministic order.
fn golden_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(GOLDEN_DIR)
        .expect("golden dir exists")
        .map(|entry| entry.expect("golden dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ad"))
        .collect();
    files.sort();
    files
}

#[test]
fn golden_files_are_canonical_and_lossless() {
    let files = golden_files();
    assert!(!files.is_empty(), "no golden files found in {GOLDEN_DIR}");

    for path in &files {
        let text = fs::read_to_string(path).expect("golden file reads as UTF-8");
        let diagram =
            ad_format::parse(&text).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
        let reserialized = ad_format::serialize(&diagram);
        assert_eq!(
            reserialized,
            text,
            "{}: re-serialization differs from the golden file",
            path.display()
        );
    }
}

#[test]
fn sample_golden_covers_scale_shapes_text_and_connectors() {
    let text = fs::read_to_string(Path::new(GOLDEN_DIR).join("sample.ad")).expect("reads");
    let diagram = ad_format::parse(&text).expect("parses");
    assert_eq!(diagram.scale(), 2);

    let mut shapes = 0;
    let mut texts = 0;
    let mut connectors = 0;
    for object in diagram.objects() {
        match object {
            diagram_model::objects::Object::Shape(_) => shapes += 1,
            diagram_model::objects::Object::Text(_) => texts += 1,
            diagram_model::objects::Object::Connector(_) => connectors += 1,
        }
    }
    assert_eq!((shapes, texts, connectors), (2, 1, 2));
}
