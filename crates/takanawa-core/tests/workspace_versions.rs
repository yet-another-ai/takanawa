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
        ("Cargo.toml", "takanawa-core"),
        ("Cargo.toml", "takanawa-http"),
        (
            "packages/takanawa-tauri/README.md",
            "tauri-plugin-takanawa\", version = \"",
        ),
        ("CMakeLists.txt", "project(Takanawa VERSION"),
        ("ports/takanawa/vcpkg.json", "\"version\""),
        ("package.json", "\"version\""),
    ];

    for (relative_path, nearby_text) in version_literals {
        let contents = fs::read_to_string(workspace_root.join(relative_path))
            .unwrap_or_else(|error| panic!("{relative_path} should be readable: {error}"));
        assert!(
            contents.contains(&format!("{nearby_text}:{workspace_version}"))
                || contents.contains(&format!(
                    "{nearby_text} = {{ version = \"{workspace_version}\""
                ))
                || contents.contains(&format!("{nearby_text}{workspace_version}"))
                || contents.contains(&format!("{nearby_text} {workspace_version}"))
                || contents.contains(&format!("{nearby_text}: \"{workspace_version}\"")),
            "{relative_path} must use workspace package version {workspace_version} near {nearby_text}; run `mise run version:sync {workspace_version}` to sync release references"
        );
    }

    let vitepress_version_variables = [
        (
            "docs/api/index.md",
            r#"implementation("ai.yetanother:takanawa-android:{{ takanawaVersion }}")"#,
        ),
        (
            "docs/guide/android.md",
            r#"implementation("ai.yetanother:takanawa-android:{{ takanawaVersion }}")"#,
        ),
        (
            "docs/guide/apple.md",
            r#".package(url: "https://github.com/yet-another-ai/takanawa.git", exact: "{{ takanawaVersion }}")"#,
        ),
        (
            "docs/guide/rust.md",
            r#"takanawa-core = "{{ takanawaVersion }}""#,
        ),
        (
            "docs/guide/rust.md",
            r#"takanawa-http = "{{ takanawaVersion }}""#,
        ),
        (
            "docs/guide/tauri.md",
            r#"takanawa-tauri = { package = "tauri-plugin-takanawa", version = "{{ takanawaVersion }}" }"#,
        ),
    ];
    for (relative_path, expected_fragment) in vitepress_version_variables {
        let contents = fs::read_to_string(workspace_root.join(relative_path))
            .unwrap_or_else(|error| panic!("{relative_path} should be readable: {error}"));
        assert!(
            contents.contains(expected_fragment),
            "{relative_path} must use the VitePress takanawaVersion variable instead of a literal release version"
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

#[test]
fn capacitor_npm_package_bundles_ios_xcframework() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("takanawa-core lives under crates/takanawa-core");
    let xtask_main = fs::read_to_string(workspace_root.join("xtask/src/main.rs"))
        .expect("xtask main should be readable");
    let package_swift =
        fs::read_to_string(workspace_root.join("packages/takanawa-capacitor/Package.swift"))
            .expect("Capacitor Package.swift should be readable");
    let package_json =
        fs::read_to_string(workspace_root.join("packages/takanawa-capacitor/package.json"))
            .expect("Capacitor package.json should be readable");
    let gitignore = fs::read_to_string(workspace_root.join(".gitignore"))
        .expect(".gitignore should be readable");
    assert!(
        package_swift.contains(r#"path: "ios/Takanawa.xcframework""#),
        "Capacitor Package.swift must load the bundled npm XCFramework by default"
    );
    assert!(
        package_swift.contains(r#"path: "ios/Sources/Takanawa""#),
        "Capacitor Package.swift must load the staged Takanawa Swift wrapper source"
    );
    assert!(
        package_json.contains(r#""ios/Takanawa.xcframework""#),
        "Capacitor npm package must include the staged XCFramework"
    );
    assert!(
        gitignore.contains("packages/takanawa-capacitor/ios/Takanawa.xcframework/"),
        "staged Capacitor XCFramework must stay out of git"
    );
    assert!(
        gitignore.contains("packages/takanawa-capacitor/ios/Sources/Takanawa/"),
        "staged Capacitor Swift wrapper source must stay out of git"
    );
    assert!(
        xtask_main.contains("prepare_capacitor_npm_package"),
        "npm publish must stage the Capacitor XCFramework before packing"
    );
}

#[test]
fn gdextension_ios_libraries_target_framework_slices() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("takanawa-core lives under crates/takanawa-core");
    let gdextension = fs::read_to_string(
        workspace_root.join("packages/takanawa-gdextension/addons/takanawa/takanawa.gdextension"),
    )
    .expect("GDExtension manifest should be readable");
    let xtask_main = fs::read_to_string(workspace_root.join("xtask/src/main.rs"))
        .expect("xtask main should be readable");

    assert!(
        gdextension.contains(
            r#"ios.arm64 = "res://addons/takanawa/bin/ios/TakanawaGDExtension.xcframework/ios-arm64/Takanawa.framework""#
        ),
        "Godot iOS device builds must load the concrete framework inside the XCFramework"
    );
    assert!(
        gdextension.contains(
            r#"ios.x86_64 = "res://addons/takanawa/bin/ios/TakanawaGDExtension.xcframework/ios-arm64_x86_64-simulator/Takanawa.framework""#
        ),
        "Godot x86_64 iOS simulator builds must load the concrete framework inside the XCFramework"
    );
    assert!(
        gdextension.contains(
            r#"ios.simulator.arm64 = "res://addons/takanawa/bin/ios/TakanawaGDExtension.xcframework/ios-arm64_x86_64-simulator/Takanawa.framework""#
        ),
        "Projects that add a simulator feature tag must be able to target the simulator framework slice"
    );
    assert!(
        !gdextension
            .contains(r#"ios = "res://addons/takanawa/bin/ios/TakanawaGDExtension.xcframework""#),
        "Godot cannot dlopen the XCFramework distribution container directly"
    );
    assert!(
        xtask_main.contains("stage_apple_framework")
            && xtask_main.contains("libtakanawa_gdextension.dylib")
            && xtask_main.contains(r#""-framework""#),
        "GDExtension Apple packaging must build XCFramework slices from concrete dynamic frameworks"
    );
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
