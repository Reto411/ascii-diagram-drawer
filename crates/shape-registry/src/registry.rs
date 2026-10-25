//! The shape registry: built-in catalog + loaded user collections, and type-id
//! resolution (C8).
//!
//! Resolution rule ([collection](../../../doc/arc42/12-concepts/collection.md)): a bare
//! type-id resolves against the built-in catalog only; `collection/name` resolves
//! against user collections only. The two namespaces are disjoint by construction —
//! no shadowing, no silent fallback; unknown ids produce typed errors.

use std::collections::BTreeMap;

use crate::error::LookupError;
use crate::types::{Collection, ShapeType};

/// The shape registry: the built-in catalog plus loaded user collections.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShapeRegistry {
    /// Built-in shapes by name (bare type-ids resolve here only).
    builtins: BTreeMap<String, ShapeType>,
    /// User collections by name (`collection/name` type-ids resolve here only).
    collections: BTreeMap<String, Collection>,
}

impl ShapeRegistry {
    /// A registry with the built-in catalog and no user collections loaded.
    #[must_use]
    pub fn builtin() -> Self {
        let builtins = crate::builtin::builtin_catalog()
            .into_iter()
            .map(|shape| (shape.name.clone(), shape))
            .collect();
        Self {
            builtins,
            collections: BTreeMap::new(),
        }
    }

    /// Loads a user collection, replacing any previously loaded collection with the same name.
    pub fn add_collection(&mut self, collection: Collection) {
        self.collections.insert(collection.name.clone(), collection);
    }

    /// Resolves a diagram type-id per the resolution rule (C8): bare token → built-in only,
    /// `collection/name` → user collections only; unknown ids produce a typed error.
    pub fn lookup(&self, type_id: &str) -> Result<&ShapeType, LookupError> {
        match type_id.split_once('/') {
            None => self
                .builtins
                .get(type_id)
                .ok_or_else(|| LookupError::UnknownBuiltin {
                    id: type_id.to_owned(),
                }),
            Some((collection, name)) => match self.collections.get(collection) {
                None => Err(LookupError::UnknownCollection {
                    collection: collection.to_owned(),
                    name: name.to_owned(),
                }),
                Some(col) => col.get(name).ok_or_else(|| LookupError::UnknownTemplate {
                    collection: collection.to_owned(),
                    name: name.to_owned(),
                }),
            },
        }
    }

    /// A loaded user collection by name.
    #[must_use]
    pub fn collection(&self, name: &str) -> Option<&Collection> {
        self.collections.get(name)
    }

    /// The built-in catalog in deterministic (alphabetical by name) order.
    pub fn builtins(&self) -> impl Iterator<Item = &ShapeType> + '_ {
        self.builtins.values()
    }

    /// The loaded user collections in deterministic (alphabetical by name) order.
    pub fn collections(&self) -> impl Iterator<Item = &Collection> + '_ {
        self.collections.values()
    }
}

#[cfg(test)]
mod tests {
    use super::ShapeRegistry;
    use crate::error::LookupError;
    use crate::templates::parse_collection;

    fn sample_collection(name: &str) -> crate::types::Collection {
        parse_collection(
            name,
            "adc 1.0\ntemplate gear 2 x 2\nab\ncd\nanchor n at 1 0\n",
        )
        .expect("valid sample collection")
        .collection
    }

    /// AC3: bare built-in names resolve through the registry.
    #[test]
    fn bare_names_resolve_to_builtins() {
        let reg = ShapeRegistry::builtin();
        assert_eq!(reg.lookup("box").expect("box is built in").type_id, "box");
        assert_eq!(
            reg.lookup("ellipse").expect("ellipse is built in").type_id,
            "ellipse"
        );
    }

    /// AC3: qualified ids resolve into loaded collections.
    #[test]
    fn qualified_ids_resolve_to_collection_templates() {
        let mut reg = ShapeRegistry::builtin();
        reg.add_collection(sample_collection("mycol"));
        let gear = reg.lookup("mycol/gear").expect("loaded template");
        assert_eq!(gear.type_id, "mycol/gear");
        assert_eq!(
            reg.collection("mycol")
                .expect("collection loaded")
                .get("gear"),
            Some(gear)
        );
    }

    /// AC3: unknown ids produce the right typed errors for each failure mode.
    #[test]
    fn unknown_ids_produce_typed_errors() {
        let mut reg = ShapeRegistry::builtin();
        reg.add_collection(sample_collection("mycol"));
        assert_eq!(
            reg.lookup("nope"),
            Err(LookupError::UnknownBuiltin {
                id: "nope".to_owned()
            })
        );
        assert_eq!(
            reg.lookup("other/gear"),
            Err(LookupError::UnknownCollection {
                collection: "other".to_owned(),
                name: "gear".to_owned()
            })
        );
        assert_eq!(
            reg.lookup("mycol/missing"),
            Err(LookupError::UnknownTemplate {
                collection: "mycol".to_owned(),
                name: "missing".to_owned()
            })
        );
    }

    /// C8: the namespaces are disjoint — a bare id never falls through to user collections,
    /// even when a collection is named like a built-in.
    #[test]
    fn namespaces_are_disjoint_with_no_fallback() {
        let mut reg = ShapeRegistry::builtin();
        // A collection literally named "box": its templates are still only reachable qualified.
        reg.add_collection(sample_collection("box"));
        assert_eq!(
            reg.lookup("box").expect("bare id stays built-in").name,
            "box"
        );
        assert_eq!(
            reg.lookup("box/gear")
                .expect("qualified reaches the collection")
                .type_id,
            "box/gear"
        );
    }

    /// Re-loading a collection name replaces the previous version.
    #[test]
    fn reloading_replaces_the_collection() {
        let mut reg = ShapeRegistry::builtin();
        reg.add_collection(sample_collection("mycol"));
        let replaced = parse_collection("mycol", "adc 1.0\ntemplate bolt 1 x 1\n.\n")
            .expect("valid sample collection")
            .collection;
        reg.add_collection(replaced);
        assert_eq!(reg.collection("mycol").expect("loaded").types.len(), 1);
        assert_eq!(
            reg.lookup("mycol/gear"),
            Err(LookupError::UnknownTemplate {
                collection: "mycol".to_owned(),
                name: "gear".to_owned()
            })
        );
        assert!(reg.lookup("mycol/bolt").is_ok());
    }

    /// Iteration is deterministic (alphabetical by name).
    #[test]
    fn iteration_is_deterministic() {
        let mut reg = ShapeRegistry::builtin();
        reg.add_collection(sample_collection("zeta"));
        reg.add_collection(sample_collection("alpha"));
        let col_names: Vec<&str> = reg.collections().map(|c| c.name.as_str()).collect();
        assert_eq!(col_names, vec!["alpha", "zeta"]);
        let builtin_names: Vec<&str> = reg.builtins().map(|s| s.name.as_str()).collect();
        assert_eq!(builtin_names, vec!["box", "ellipse"]);
    }
}
