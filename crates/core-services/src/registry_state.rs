//! Shared catalog state behind the ADR 009 seam (S-017).
//!
//! One [`ShapeRegistry`] instance, visible to **both** the renderer and the catalog
//! service: a template registered through [`ShapeCatalog::register`](ui_api::ShapeCatalog::register)
//! is immediately listed by the sidebar *and* resolvable by the renderer, with no
//! re-composition. This is the only interior mutability in `core-services` —
//! registration was pinned as "v1+" in ADR 009 and lands now (ADR 009 v6).

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use shape_registry::{
    AnchorSet, Collection, GeometryKind, ShapeRegistry, ShapeType, TemplateGrid,
    serialize_collection,
};
use ui_api::{CatalogError, NewTemplate};

/// A registry shared between the services that read it. Cloning the handle shares
/// state (the registry itself is a cheap deep clone; the `Arc` makes it one object).
#[derive(Debug, Clone)]
pub struct SharedRegistry {
    inner: Arc<Mutex<ShapeRegistry>>,
    /// Where collection `.adc` files are persisted; `None` → [`register`](Self::register)
    /// fails with [`CatalogError::Io`] (built-in-only compositions, e.g. `web()`).
    collections_dir: Option<PathBuf>,
}

impl SharedRegistry {
    /// A standalone registry — no persistence directory, registration unavailable.
    #[must_use]
    pub fn standalone(registry: ShapeRegistry) -> Self {
        Self {
            inner: Arc::new(Mutex::new(registry)),
            collections_dir: None,
        }
    }

    /// A shared registry that persists registered templates into `dir` as
    /// `<collection>.adc` (S-017 AC3).
    #[must_use]
    pub fn with_collections_dir(registry: ShapeRegistry, dir: impl Into<PathBuf>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(registry)),
            collections_dir: Some(dir.into()),
        }
    }

    /// A deep snapshot of the current registry (cloned under the lock).
    #[must_use]
    pub fn snapshot(&self) -> ShapeRegistry {
        self.lock().clone()
    }

    /// Registers (or replaces) `template` in `collection`: validates it into a
    /// [`ShapeType`], persists the collection to `<dir>/<collection>.adc`, and updates
    /// the shared registry — every service reading it sees the template at once.
    ///
    /// # Errors
    /// [`CatalogError::Invalid`] for bad template data (checked before any IO);
    /// [`CatalogError::Io`] when no collections directory is configured or the file
    /// cannot be written. The registry is left unchanged on any failure.
    pub fn register(&self, collection: &str, template: NewTemplate) -> Result<(), CatalogError> {
        let dir = self
            .collections_dir
            .clone()
            .ok_or_else(|| CatalogError::Io {
                message: "no collections directory is configured in this build".to_owned(),
            })?;

        // Validation happens before any IO — a bad template never touches the disk.
        let shape = self.build_shape(collection, &template)?;

        // Read the current collection (if any) so other templates survive the save.
        let existing = {
            let guard = self.lock();
            guard.collection(collection).cloned()
        };

        let updated = match existing {
            Some(mut col) => {
                // Replace in place (declaration order of the other templates is kept);
                // a new name appends at the end — deterministic either way.
                match col.types.iter_mut().find(|t| t.name == shape.name) {
                    Some(slot) => *slot = shape,
                    None => col.types.push(shape),
                }
                col
            }
            None => Collection {
                name: collection.to_owned(),
                types: vec![shape],
            },
        };

        // Persist first; the registry is only updated once the file is on disk.
        let path = dir.join(format!("{collection}.adc"));
        std::fs::write(&path, serialize_collection(&updated)).map_err(|e| CatalogError::Io {
            message: format!("could not write {}: {e}", path.display()),
        })?;

        self.lock().add_collection(updated);
        Ok(())
    }

    /// Validates the seam-level [`NewTemplate`] into a registry [`ShapeType`].
    fn build_shape(
        &self,
        collection: &str,
        template: &NewTemplate,
    ) -> Result<ShapeType, CatalogError> {
        invalid_token(collection, "collection name")?;
        invalid_token(&template.name, "template name")?;

        // Ragged rows are right-padded to the longest row (the parser's rule), so a
        // grid of all spaces is still a valid (blank) pattern.
        let padded = pad_rows(&template.rows);
        if padded.iter().all(|r| r.chars().all(|c| c == ' ')) {
            return Err(CatalogError::Invalid {
                message: "the glyph grid is empty — type at least one character".to_owned(),
            });
        }
        let rows: Vec<&str> = padded.iter().map(String::as_str).collect();
        let glyph_pattern = TemplateGrid::from_rows(&rows).map_err(|e| CatalogError::Invalid {
            message: e.to_string(),
        })?;

        let mut anchor_set = AnchorSet::new();
        if template.anchors.is_empty() {
            return Err(CatalogError::Invalid {
                message: "at least one anchor is required — connectors bind to anchors".to_owned(),
            });
        }
        for anchor in &template.anchors {
            invalid_token(&anchor.name, "anchor name")?;
            if anchor.position.x >= glyph_pattern.width()
                || anchor.position.y >= glyph_pattern.height()
            {
                return Err(CatalogError::Invalid {
                    message: format!(
                        "anchor '{}' at ({}, {}) is outside the {}x{} grid",
                        anchor.name,
                        anchor.position.x,
                        anchor.position.y,
                        glyph_pattern.width(),
                        glyph_pattern.height()
                    ),
                });
            }
            anchor_set
                .add(
                    &anchor.name,
                    shape_registry::CellPos {
                        x: anchor.position.x,
                        y: anchor.position.y,
                    },
                )
                .map_err(|e| CatalogError::Invalid {
                    message: e.to_string(),
                })?;
        }

        Ok(ShapeType {
            type_id: format!("{}/{}", collection, template.name),
            name: template.name.clone(),
            glyph_pattern,
            anchor_set,
            geometry: GeometryKind::Pattern,
            resize_rules: None,
        })
    }

    fn lock(&self) -> MutexGuard<'_, ShapeRegistry> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Right-pads `rows` to the longest row (the parser's rule for short grid rows).
pub(crate) fn pad_rows(rows: &[String]) -> Vec<String> {
    let width = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0);
    rows.iter()
        .map(|r| {
            let mut s = r.clone();
            for _ in 0..width - s.chars().count() {
                s.push(' ');
            }
            s
        })
        .collect()
}

/// Rejects empty names and names with whitespace or `/` — both would break the
/// `collection/name` type-id grammar (C8).
fn invalid_token(name: &str, what: &str) -> Result<(), CatalogError> {
    if name.is_empty() || name.contains('/') || name.chars().any(|c| c.is_whitespace()) {
        return Err(CatalogError::Invalid {
            message: format!("{what} '{name}' must be a single token without '/' or whitespace"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path as StdPath;
    use ui_api::AnchorInfo;

    fn draft(rows: Vec<&str>, anchors: &[(&str, u32, u32)]) -> NewTemplate {
        NewTemplate {
            name: "star".to_owned(),
            rows: rows.into_iter().map(String::from).collect(),
            anchors: anchors
                .iter()
                .map(|(n, x, y)| AnchorInfo {
                    name: (*n).to_owned(),
                    position: diagram_model::geometry::CellPos { x: *x, y: *y },
                })
                .collect(),
        }
    }

    fn shared(dir: &StdPath) -> SharedRegistry {
        SharedRegistry::with_collections_dir(ShapeRegistry::builtin(), dir)
    }

    /// S-017 AC3 — registration persists a new collection file and the template is
    /// immediately resolvable through the same shared state.
    #[test]
    fn register_persists_and_updates_the_shared_registry() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = shared(dir.path());

        state
            .register("stars", draft(vec!["###", "#.#", "###"], &[("n", 1, 0)]))
            .expect("valid template");

        // The file exists with the canonical bytes…
        let text = std::fs::read_to_string(dir.path().join("stars.adc")).expect("written");
        assert_eq!(
            text,
            "adc 1.0\ntemplate star 3 x 3\n###\n#.#\n###\nanchor n at 1 0\n"
        );
        // …and the shared registry resolves it (renderer and catalog both see it).
        assert!(state.snapshot().lookup("stars/star").is_ok());
    }

    /// S-017 AC3 — registering into an existing collection keeps its other templates
    /// and replaces same-named ones in place.
    #[test]
    fn register_upserts_within_an_existing_collection() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("stars.adc"),
            "adc 1.0\ntemplate old 2 x 2\nab\ncd\nanchor w at 0 0\n",
        )
        .expect("seed file");

        let mut registry = ShapeRegistry::builtin();
        shape_registry::parse_collection(
            "stars",
            &std::fs::read_to_string(dir.path().join("stars.adc")).unwrap(),
        )
        .map(|f| registry.add_collection(f.collection))
        .expect("seed parses");
        let state = SharedRegistry::with_collections_dir(registry, dir.path());

        // A new name appends; the old template survives.
        state
            .register(
                "stars",
                NewTemplate {
                    name: "new".to_owned(),
                    rows: vec!["x".to_owned()],
                    anchors: vec![AnchorInfo {
                        name: "e".into(),
                        position: diagram_model::geometry::CellPos { x: 0, y: 0 },
                    }],
                },
            )
            .expect("valid");
        let text = std::fs::read_to_string(dir.path().join("stars.adc")).unwrap();
        assert!(text.contains("template old 2 x 2"));
        assert!(text.contains("template new 1 x 1"));

        // A same-named template replaces in place (still exactly two templates).
        state
            .register(
                "stars",
                NewTemplate {
                    name: "old".to_owned(),
                    rows: vec!["z".to_owned()],
                    anchors: vec![AnchorInfo {
                        name: "s".into(),
                        position: diagram_model::geometry::CellPos { x: 0, y: 0 },
                    }],
                },
            )
            .expect("valid");
        let text = std::fs::read_to_string(dir.path().join("stars.adc")).unwrap();
        assert_eq!(text.matches("template ").count(), 2);
        assert!(!text.contains("ab"));
    }

    /// AC2 — validation failures are typed, happen before any IO, and never touch
    /// the registry: no anchors, out-of-bounds anchor, non-printable cell, bad name.
    #[test]
    fn invalid_templates_are_rejected_before_any_io() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = shared(dir.path());

        assert!(matches!(
            state.register("stars", draft(vec!["x"], &[])),
            Err(CatalogError::Invalid { .. })
        ));
        assert!(matches!(
            state.register("stars", draft(vec!["x"], &[("n", 5, 0)])),
            Err(CatalogError::Invalid { .. })
        ));
        assert!(matches!(
            state.register("stars", draft(vec!["é"], &[("n", 0, 0)])),
            Err(CatalogError::Invalid { .. })
        ));
        assert!(matches!(
            state.register("bad name", draft(vec!["x"], &[("n", 0, 0)])),
            Err(CatalogError::Invalid { .. })
        ));
        // Nothing was written and the registry is untouched.
        assert!(dir.path().read_dir().unwrap().next().is_none());
        assert!(state.snapshot().collections().next().is_none());
    }

    /// Duplicate anchor names are rejected (the AnchorSet invariant).
    #[test]
    fn duplicate_anchor_names_are_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = shared(dir.path());
        assert!(matches!(
            state.register("stars", draft(vec!["x"], &[("n", 0, 0), ("n", 0, 0)])),
            Err(CatalogError::Invalid { message }) if message.contains("already declared")
        ));
    }

    /// No collections directory → a clear IO error (built-in-only compositions).
    #[test]
    fn register_without_a_collections_dir_is_an_io_error() {
        let state = SharedRegistry::standalone(ShapeRegistry::builtin());
        assert!(matches!(
            state.register("stars", draft(vec!["x"], &[("n", 0, 0)])),
            Err(CatalogError::Io { .. })
        ));
    }

    /// Ragged rows are right-padded to the longest row (the parser's rule).
    #[test]
    fn ragged_rows_are_right_padded() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = shared(dir.path());
        state
            .register("c", draft(vec!["ab", "abcd"], &[("w", 0, 1)]))
            .expect("pads fine");
        assert_eq!(
            state
                .snapshot()
                .lookup("c/star")
                .expect("resolved")
                .glyph_pattern
                .row(0)
                .expect("row 0"),
            "ab  "
        );
    }
}
