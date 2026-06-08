use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use serde_json::Value;

#[test]
fn workspace_packages_share_one_version() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("takanawa-core lives under crates/takanawa-core");
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(workspace_root)
        .output()
        .expect("cargo metadata should run");

    assert!(
        output.status.success(),
        "cargo metadata failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let metadata: Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata output should be valid JSON");
    let workspace_members = metadata["workspace_members"]
        .as_array()
        .expect("metadata has workspace_members")
        .iter()
        .map(|member| {
            member
                .as_str()
                .expect("workspace member id is a string")
                .to_owned()
        })
        .collect::<BTreeSet<_>>();

    let packages = metadata["packages"]
        .as_array()
        .expect("metadata has packages");
    let mut versions = BTreeMap::new();

    for package in packages {
        let id = package["id"]
            .as_str()
            .expect("package id is a string")
            .to_owned();
        if !workspace_members.contains(&id) {
            continue;
        }

        let name = package["name"].as_str().expect("package name is a string");
        let version = package["version"]
            .as_str()
            .expect("package version is a string");
        versions.insert(name.to_owned(), version.to_owned());
    }

    let unique_versions = versions.values().collect::<BTreeSet<_>>();
    assert_eq!(
        unique_versions.len(),
        1,
        "workspace package versions must match exactly: {versions:#?}"
    );
}
