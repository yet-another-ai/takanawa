use std::env;
use std::fs;
use std::path::Path;
use std::process::Stdio;
use std::{thread, time};

use crate::support::{Result, output_text, repo_command, repo_root, run_command};

pub(crate) fn github_release() -> Result<()> {
    let tag = env::var("GITHUB_REF_NAME").map_err(|_| "GITHUB_REF_NAME is required")?;
    let artifacts_dir = repo_root().join("target/release-artifacts");
    let mut artifacts = Vec::new();
    for entry in fs::read_dir(&artifacts_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            artifacts.push(entry.path());
        }
    }
    artifacts.sort();
    if artifacts.is_empty() {
        return Err(format!("no release artifacts found in {}", artifacts_dir.display()).into());
    }

    let release_exists = repo_command("gh")
        .args(["release", "view", tag.as_str()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?
        .success();

    if release_exists {
        let mut command = repo_command("gh");
        command.args(["release", "upload", tag.as_str()]);
        for artifact in artifacts {
            command.arg(artifact);
        }
        command.arg("--clobber");
        run_command(&mut command)
    } else {
        let mut command = repo_command("gh");
        command.args([
            "release",
            "create",
            tag.as_str(),
            "--title",
            tag.as_str(),
            "--generate-notes",
        ]);
        for artifact in artifacts {
            command.arg(artifact);
        }
        run_command(&mut command)
    }
}

pub(crate) fn publish_crates() -> Result<()> {
    let token = env::var("CARGO_REGISTRY_TOKEN")
        .map_err(|_| "CARGO_REGISTRY_TOKEN is required to publish crates")?;
    if token.is_empty() {
        return Err("CARGO_REGISTRY_TOKEN is required to publish crates".into());
    }

    let crates = env::var("PUBLISH_CRATES").unwrap_or_else(|_| {
        "takanawa-core takanawa-http takanawa-ffi takanawa-cli tauri-plugin-takanawa".to_owned()
    });
    for krate in crates.split_whitespace() {
        publish_one_crate(krate)?;
    }
    Ok(())
}

fn publish_one_crate(krate: &str) -> Result<()> {
    let version = crate_version(krate)?;
    if crate_version_exists(krate, &version)? {
        println!(
            "::notice title=Already published::{krate} {version} already exists on crates.io; skipping."
        );
        return Ok(());
    }

    run_command(repo_command("cargo").args(["publish", "--locked", "-p", krate]))?;
    wait_for_crate_version(krate, &version)
}

fn crate_version(krate: &str) -> Result<String> {
    let metadata = output_text(repo_command("cargo").args([
        "metadata",
        "--no-deps",
        "--format-version",
        "1",
    ]))?;
    let metadata: serde_json::Value = serde_json::from_str(&metadata)?;
    let packages = metadata
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or("cargo metadata did not contain packages")?;
    for package in packages {
        if package.get("name").and_then(serde_json::Value::as_str) == Some(krate) {
            return package
                .get("version")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| format!("{krate} is missing a version").into());
        }
    }
    Err(format!("crate {krate} was not found in cargo metadata").into())
}

fn crate_version_exists(krate: &str, version: &str) -> Result<bool> {
    let status = repo_command("curl")
        .args([
            "-fsS",
            "-H",
            "User-Agent: takanawa-ci",
            format!("https://crates.io/api/v1/crates/{krate}/{version}").as_str(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    Ok(status.success())
}

fn wait_for_crate_version(krate: &str, version: &str) -> Result<()> {
    for attempt in 1..=60 {
        if crate_version_exists(krate, version)? {
            println!("{krate} {version} is visible on crates.io");
            return Ok(());
        }
        println!("Waiting for {krate} {version} to become visible on crates.io ({attempt}/60)");
        thread::sleep(time::Duration::from_secs(10));
    }
    Err(format!("{krate} {version} did not become visible on crates.io").into())
}

pub(crate) fn sync_version(version: Option<String>) -> Result<()> {
    let version = match version {
        Some(version) => version,
        None => workspace_version()?,
    };
    sync_cargo_workspace_versions(&version)?;
    replace_project_version("CMakeLists.txt", "project(Takanawa VERSION ", &version)?;
    sync_json_version("ports/takanawa/vcpkg.json", &version)?;
    sync_json_version("package.json", &version)?;

    for entry in fs::read_dir(repo_root().join("packages"))? {
        let manifest = entry?.path().join("package.json");
        if manifest.is_file() {
            sync_json_version(manifest, &version)?;
        }
    }
    replace_between_all_in_file(
        "packages/takanawa-csharp/src/YetAnotherAI.Takanawa/YetAnotherAI.Takanawa.csproj",
        "<Version>",
        "</Version>",
        &version,
    )?;

    replace_between_all_in_file(
        "packages/takanawa-capacitor/android/build.gradle",
        "def takanawaVersion = \"",
        "\"",
        &version,
    )?;
    replace_between_all_in_file(
        "README.md",
        "implementation(\"ai.yetanother:takanawa-android:",
        "\")",
        &version,
    )?;
    replace_between_all_in_file(
        "packages/takanawa-tauri/README.md",
        "takanawa-tauri = { package = \"tauri-plugin-takanawa\", version = \"",
        "\"",
        &version,
    )?;

    println!("Synced release version references to {version}");
    Ok(())
}

pub(crate) fn workspace_version() -> Result<String> {
    let path = repo_root().join("Cargo.toml");
    let document = read_toml_document(&path)?;
    document
        .get("workspace")
        .and_then(toml_edit::Item::as_table)
        .and_then(|workspace| workspace.get("package"))
        .and_then(toml_edit::Item::as_table)
        .and_then(|package| package.get("version"))
        .and_then(toml_edit::Item::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "missing [workspace.package] version in Cargo.toml".into())
}

fn sync_cargo_workspace_versions(version: &str) -> Result<()> {
    let path = repo_root().join("Cargo.toml");
    let mut document = read_toml_document(&path)?;
    sync_workspace_package_version(&mut document, version)?;
    for (dependency, expected_path) in [
        ("takanawa-core", "crates/takanawa-core"),
        ("takanawa-http", "crates/takanawa-http"),
    ] {
        sync_workspace_dependency_version(&mut document, dependency, expected_path, version)?;
    }
    fs::write(path, document.to_string())?;
    Ok(())
}

fn sync_workspace_package_version(
    document: &mut toml_edit::DocumentMut,
    version: &str,
) -> Result<()> {
    let workspace_package = document
        .get_mut("workspace")
        .and_then(toml_edit::Item::as_table_mut)
        .and_then(|workspace| workspace.get_mut("package"))
        .and_then(toml_edit::Item::as_table_mut)
        .ok_or("missing [workspace.package] in Cargo.toml")?;
    let package_version = workspace_package
        .get_mut("version")
        .ok_or("missing [workspace.package] version in Cargo.toml")?;
    *package_version = toml_edit::value(version);
    Ok(())
}

fn read_toml_document(path: &Path) -> Result<toml_edit::DocumentMut> {
    let content = fs::read_to_string(path)?;
    content
        .parse::<toml_edit::DocumentMut>()
        .map_err(|error| format!("failed to parse {}: {error}", path.display()).into())
}

fn sync_workspace_dependency_version(
    document: &mut toml_edit::DocumentMut,
    dependency: &str,
    expected_path: &str,
    version: &str,
) -> Result<()> {
    let dependencies = document
        .get_mut("workspace")
        .and_then(toml_edit::Item::as_table_mut)
        .and_then(|workspace| workspace.get_mut("dependencies"))
        .and_then(toml_edit::Item::as_table_mut)
        .ok_or("missing [workspace.dependencies] in Cargo.toml")?;
    let dependency_item = dependencies
        .get_mut(dependency)
        .ok_or_else(|| format!("missing workspace dependency {dependency}"))?;
    let dependency_table = dependency_item
        .as_inline_table_mut()
        .ok_or_else(|| format!("workspace dependency {dependency} must be an inline table"))?;
    let dependency_path = dependency_table
        .get("path")
        .and_then(toml_edit::Value::as_str)
        .ok_or_else(|| format!("workspace dependency {dependency} is missing path"))?;
    if dependency_path != expected_path {
        return Err(format!(
            "workspace dependency {dependency} points to {dependency_path}, expected {expected_path}"
        )
        .into());
    }
    let dependency_version = dependency_table
        .get_mut("version")
        .ok_or_else(|| format!("workspace dependency {dependency} is missing version"))?;
    *dependency_version = toml_edit::Value::from(version);
    Ok(())
}

fn replace_project_version(path: impl AsRef<Path>, prefix: &str, version: &str) -> Result<()> {
    let path = repo_root().join(path);
    let content = fs::read_to_string(&path)?;
    let content = content
        .lines()
        .map(|line| {
            if let Some(start) = line.find(prefix) {
                let value_start = start + prefix.len();
                let value_end = line[value_start..]
                    .find(|ch: char| ch.is_whitespace() || ch == ')')
                    .map(|end| value_start + end)
                    .unwrap_or(line.len());
                format!("{}{}{}", &line[..value_start], version, &line[value_end..])
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path, format!("{content}\n"))?;
    Ok(())
}

fn sync_json_version(path: impl AsRef<Path>, version: &str) -> Result<()> {
    let path = repo_root().join(path);
    let content = fs::read_to_string(&path)?;
    let mut manifest: serde_json::Value = serde_json::from_str(&content)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    let manifest_object = manifest
        .as_object_mut()
        .ok_or_else(|| format!("{} must contain a JSON object", path.display()))?;
    let manifest_version = manifest_object
        .get_mut("version")
        .ok_or_else(|| format!("{} is missing version", path.display()))?;
    if !manifest_version.is_string() {
        return Err(format!("{} version must be a string", path.display()).into());
    }
    *manifest_version = serde_json::Value::String(version.to_owned());
    let mut output = serde_json::to_string_pretty(&manifest)?;
    output.push('\n');
    fs::write(path, output)?;
    Ok(())
}

fn replace_between_all_in_file(
    path: impl AsRef<Path>,
    start_marker: &str,
    end_marker: &str,
    value: &str,
) -> Result<()> {
    let path = repo_root().join(path);
    if !path.is_file() {
        return Err(format!("missing {}", path.display()).into());
    }
    let mut content = fs::read_to_string(&path)?;
    let mut search_from = 0;
    let mut replaced = false;
    while let Some(start) = content[search_from..].find(start_marker) {
        let value_start = search_from + start + start_marker.len();
        let Some(end) = content[value_start..].find(end_marker) else {
            break;
        };
        let value_end = value_start + end;
        content.replace_range(value_start..value_end, value);
        replaced = true;
        search_from = value_start + value.len() + end_marker.len();
    }
    if !replaced {
        return Err(format!(
            "missing version marker {start_marker:?} in {}",
            path.display()
        )
        .into());
    }
    fs::write(path, content)?;
    Ok(())
}
