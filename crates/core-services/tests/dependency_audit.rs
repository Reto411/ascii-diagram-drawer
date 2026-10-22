//! T5 — machine-checked dependency DAG audit ([§8.3](../../../doc/arc42/08-building-block-view.md), [ADR 011](../../../doc/adr/011-domain-driven-design.md)).
//!
//! Parses every workspace member's `Cargo.toml` and asserts that the declared
//! *normal* dependencies between workspace crates are a subset of the allow-list
//! pinned at scaffolding (S-001). The allow-list is acyclic, so any graph made of
//! its subsets is acyclic. Dev-dependencies are excluded: they never create a
//! runtime coupling edge (this test itself needs `toml` as a dev-dependency).
#![allow(
    clippy::panic,
    reason = "test code: panicking with the offending manifest/edge in the message is the correct failure mode"
)]
#![allow(
    clippy::tests_outside_test_module,
    reason = "this file is an integration test — the whole file is test code"
)]
#![allow(
    clippy::indexing_slicing,
    reason = "toml Value indexing by fixed manifest keys; ADR 010 names this as an expected test-code allow"
)]
#![allow(
    clippy::single_call_fn,
    reason = "`allowed_edges` is a one-shot lookup table over the nine packages"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test code: panicking on an unreadable or unparseable manifest is the correct behavior"
)]

use std::fs;

/// Workspace members expected in the root `Cargo.toml`, exactly (§8.3 layout).
const EXPECTED_MEMBERS: &[&str] = &[
    "crates/diagram-model",
    "crates/ad-format",
    "crates/shape-registry",
    "crates/render-pipeline",
    "crates/exporters",
    "crates/ascii-import",
    "crates/ui-api",
    "crates/core-services",
    "gui-app",
];

/// §8.3 coupling strategy — the only permitted workspace-internal edges.
fn allowed_edges(package: &str) -> &'static [&'static str] {
    match package {
        "diagram-model" | "shape-registry" => &[],
        "ad-format" => &["diagram-model"],
        "render-pipeline" => &["diagram-model", "shape-registry"],
        "exporters" => &["render-pipeline"],
        "ascii-import" => &["diagram-model", "shape-registry", "render-pipeline"],
        "ui-api" => &["diagram-model", "render-pipeline"],
        "core-services" => &[
            "ui-api",
            "diagram-model",
            "ad-format",
            "shape-registry",
            "render-pipeline",
            "exporters",
            "ascii-import",
        ],
        "gui-app" => &["ui-api", "core-services", "diagram-model"],
        other => panic!("audit has no allow-list entry for package `{other}`"),
    }
}

/// This test lives in `crates/core-services`; the workspace root is two levels up.
fn workspace_root() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let without_crate = manifest_dir
        .strip_suffix("/core-services")
        .unwrap_or(manifest_dir);
    String::from(
        without_crate
            .strip_suffix("/crates")
            .unwrap_or(without_crate),
    )
}

fn read_manifest(relative: &str) -> String {
    let path = if relative.is_empty() {
        format!("{}/Cargo.toml", workspace_root())
    } else {
        format!("{}/{relative}/Cargo.toml", workspace_root())
    };
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("cannot read {path}: {err}"))
}

#[test]
fn workspace_members_match_the_expected_set() {
    let root: toml::Value = toml::from_str(&read_manifest("")).expect("root manifest must parse");
    let members = root["workspace"]["members"]
        .as_array()
        .expect("root manifest must declare workspace.members");
    let names: Vec<&str> = members
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();

    assert_eq!(
        names.len(),
        EXPECTED_MEMBERS.len(),
        "unexpected member count"
    );
    for expected in EXPECTED_MEMBERS {
        assert!(
            names.contains(expected),
            "workspace member `{expected}` is missing from the root manifest"
        );
    }
}

#[test]
fn every_internal_dependency_edge_is_on_the_allow_list() {
    let manifests: Vec<(String, toml::Value)> = EXPECTED_MEMBERS
        .iter()
        .map(|member| {
            let manifest: toml::Value =
                toml::from_str(&read_manifest(member)).expect("member manifest must parse");
            let package = String::from(
                manifest["package"]["name"]
                    .as_str()
                    .unwrap_or_else(|| panic!("{member}: manifest has no package.name")),
            );
            (package, manifest)
        })
        .collect();
    let names: Vec<&str> = manifests.iter().map(|entry| entry.0.as_str()).collect();

    for entry in &manifests {
        let package = &entry.0;
        let manifest = &entry.1;
        let deps = manifest
            .get("dependencies")
            .and_then(toml::Value::as_table)
            .cloned()
            .unwrap_or_default();
        let internal: Vec<&str> = deps
            .keys()
            .filter(|name| names.contains(&name.as_str()))
            .map(String::as_str)
            .collect();

        let allowed = allowed_edges(package);
        for dep in &internal {
            assert!(
                allowed.contains(dep),
                "`{package}` depends on workspace crate `{dep}`, which is not on the \
                 §8.3 allow-list (allowed: {allowed:?}) — cross-context communication \
                 needs a context mapping, see ADR 011"
            );
        }
    }
}
