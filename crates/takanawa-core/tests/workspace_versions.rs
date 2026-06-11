use std::collections::{BTreeMap, BTreeSet};
use std::fs;
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

#[test]
fn published_version_references_match_workspace_version() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("takanawa-core lives under crates/takanawa-core");
    let cargo_toml = fs::read_to_string(workspace_root.join("Cargo.toml"))
        .expect("workspace Cargo.toml should be readable");
    let workspace_version =
        workspace_package_version(&cargo_toml).expect("workspace package version should exist");

    let version_literals = [
        ("README.md", "takanawa-android"),
        ("docs/api/index.md", "takanawa-android"),
        ("docs/guide/getting-started.md", "takanawa-android"),
        ("Cargo.toml", "takanawa-core"),
        ("Cargo.toml", "takanawa-http"),
        ("CMakeLists.txt", "project(Takanawa VERSION"),
        ("ports/takanawa/vcpkg.json", "\"version\""),
    ];

    for (relative_path, nearby_text) in version_literals {
        let contents = fs::read_to_string(workspace_root.join(relative_path))
            .unwrap_or_else(|error| panic!("{relative_path} should be readable: {error}"));
        assert!(
            contents.contains(&format!("{nearby_text}:{workspace_version}"))
                || contents.contains(&format!(
                    "{nearby_text} = {{ version = \"{workspace_version}\""
                ))
                || contents.contains(&format!("{nearby_text} {workspace_version}"))
                || contents.contains(&format!("{nearby_text}: \"{workspace_version}\"")),
            "{relative_path} must use workspace package version {workspace_version} near {nearby_text}; run `mise run version:sync` after changing the workspace version"
        );
    }

    let generated_version_files = [
        "android/takanawa-android/build.gradle.kts",
        "fixtures/android-maven-local-smoke/build.gradle.kts",
        "gradle.properties",
    ];
    for relative_path in generated_version_files {
        let contents = fs::read_to_string(workspace_root.join(relative_path))
            .unwrap_or_else(|error| panic!("{relative_path} should be readable: {error}"));
        assert!(
            !contents.contains(&format!("\"{workspace_version}\""))
                && !contents.contains(&format!("={workspace_version}")),
            "{relative_path} should derive the release version from Cargo.toml"
        );
    }
}

fn workspace_package_version(cargo_toml: &str) -> Option<String> {
    let mut in_workspace_package = false;
    for raw_line in cargo_toml.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_workspace_package = line == "[workspace.package]";
        } else if in_workspace_package && line.starts_with("version") {
            return line.split('"').nth(1).map(str::to_owned);
        }
    }
    None
}
