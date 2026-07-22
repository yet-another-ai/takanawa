use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::apple::verify_apple_xcframework;
use crate::support::{Result, copy_dir, repo_command, repo_root, run_command};

const TAKANAWA_NODE_ARTIFACT_DIR: &str = "target/takanawa-node-npm-artifacts";
const TAKANAWA_NODE_PACKAGE_DIR: &str = "packages/takanawa-node";
const TAKANAWA_NODE_NATIVE_FILES: [&str; 3] = [
    "takanawa.linux-x64-gnu.node",
    "takanawa.darwin-arm64.node",
    "takanawa.win32-x64-msvc.node",
];

pub(crate) fn npm_publish(mode: &str) -> Result<()> {
    if mode != "dry-run" && mode != "publish" {
        return Err("usage: xtask npm-publish <dry-run|publish>".into());
    }

    let root = repo_root();
    let pnpm_cache = env::var_os("PNPM_CONFIG_CACHE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target/pnpm-cache"));
    fs::create_dir_all(&pnpm_cache)?;

    let packages = publishable_npm_packages()?;
    if packages.is_empty() {
        println!("::notice title=No npm packages::No publishable npm packages were found.");
        return Ok(());
    }

    if packages
        .iter()
        .any(|package| package.name == "takanawa-capacitor")
    {
        prepare_capacitor_npm_package()?;
    }

    let publish_takanawa_node = mode == "publish"
        && packages
            .iter()
            .any(|package| package.name == "takanawa-node");
    if publish_takanawa_node {
        prepare_takanawa_node_npm_package()?;
    }

    for package in &packages {
        let build_script = if publish_takanawa_node && package.name == "takanawa-node" {
            "build:ts"
        } else {
            "build"
        };
        println!("::group::pnpm --filter {} run {build_script}", package.name);
        run_command(pnpm_command(&pnpm_cache).args([
            "--filter",
            package.name.as_str(),
            "run",
            build_script,
        ]))?;
        println!("::endgroup::");
    }

    if publish_takanawa_node {
        verify_takanawa_node_npm_package()?;
    }

    for package in &packages {
        if mode == "dry-run" {
            println!("::group::pnpm pack --dry-run {}", package.dir);
            run_command(
                pnpm_command(&pnpm_cache)
                    .current_dir(root.join(&package.dir))
                    .args(["pack", "--dry-run"]),
            )?;
            println!("::endgroup::");
            continue;
        }

        if npm_package_version_exists(&pnpm_cache, &package.name, &package.version)? {
            println!(
                "::notice title=Already published::{} {} already exists on npm; skipping.",
                package.name, package.version
            );
            continue;
        }

        let mut args = vec!["publish", "--provenance", "--no-git-checks"];
        if package.name.starts_with('@') {
            args.push("--access");
            args.push("public");
        }
        println!("::group::pnpm publish {}@{}", package.name, package.version);
        run_command(
            pnpm_command(&pnpm_cache)
                .current_dir(root.join(&package.dir))
                .args(args),
        )?;
        println!("::endgroup::");
    }

    Ok(())
}

pub(crate) fn package_takanawa_node_npm() -> Result<()> {
    let root = repo_root();
    let pnpm_cache = env::var_os("PNPM_CONFIG_CACHE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target/pnpm-cache"));
    fs::create_dir_all(&pnpm_cache)?;

    prepare_takanawa_node_npm_package()?;

    println!("::group::Build takanawa-node TypeScript wrapper");
    run_command(pnpm_command(&pnpm_cache).args(["--filter", "takanawa-node", "run", "build:ts"]))?;
    println!("::endgroup::");

    verify_takanawa_node_npm_package()?;

    let pack_dir = root.join("target/takanawa-node-npm-package");
    if pack_dir.is_dir() {
        fs::remove_dir_all(&pack_dir)?;
    }
    fs::create_dir_all(&pack_dir)?;

    println!("::group::pnpm pack packages/takanawa-node");
    run_command(
        pnpm_command(&pnpm_cache)
            .current_dir(root.join(TAKANAWA_NODE_PACKAGE_DIR))
            .arg("pack")
            .arg("--pack-destination")
            .arg(&pack_dir),
    )?;
    println!("::endgroup::");

    Ok(())
}

fn pnpm_command(pnpm_cache: &Path) -> Command {
    let mut command = repo_command("pnpm");
    command.env("PNPM_CONFIG_CACHE_DIR", pnpm_cache);
    command
}

fn npm_package_version_exists(pnpm_cache: &Path, name: &str, version: &str) -> Result<bool> {
    let status = pnpm_command(pnpm_cache)
        .args(["view", format!("{name}@{version}").as_str(), "version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    Ok(status.success())
}

#[derive(Debug)]
struct NpmPackage {
    dir: String,
    name: String,
    version: String,
}

fn publishable_npm_packages() -> Result<Vec<NpmPackage>> {
    let packages_dir = repo_root().join("packages");
    if !packages_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut manifests = Vec::new();
    for entry in fs::read_dir(packages_dir)? {
        let entry = entry?;
        let manifest = entry.path().join("package.json");
        if manifest.is_file() {
            manifests.push(manifest);
        }
    }
    manifests.sort();

    let mut packages = Vec::new();
    for manifest_path in manifests {
        let manifest_text = fs::read_to_string(&manifest_path)?;
        let manifest: serde_json::Value = serde_json::from_str(&manifest_text)?;
        let dir = manifest_path
            .parent()
            .expect("package manifest should have a parent")
            .strip_prefix(repo_root())?
            .to_string_lossy()
            .replace('\\', "/");

        if manifest
            .get("private")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        {
            println!("::notice title=Skipping private package::{dir}");
            continue;
        }

        let name = manifest
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("{dir}/package.json is missing name"))?
            .to_owned();
        let version = manifest
            .get("version")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("{dir}/package.json is missing version"))?
            .to_owned();
        packages.push(NpmPackage { dir, name, version });
    }

    Ok(packages)
}

fn prepare_takanawa_node_npm_package() -> Result<()> {
    let root = repo_root();
    let artifact_dir = root.join(TAKANAWA_NODE_ARTIFACT_DIR);
    let package_dir = root.join(TAKANAWA_NODE_PACKAGE_DIR);
    let mut required_files = TAKANAWA_NODE_NATIVE_FILES.to_vec();
    required_files.push("index.js");

    for file_name in required_files {
        let source = artifact_dir.join(file_name);
        if !source.is_file() {
            return Err(format!(
                "missing takanawa-node release artifact {}; build and download every supported platform artifact first",
                source.display()
            )
            .into());
        }
        fs::copy(&source, package_dir.join(file_name))?;
    }

    println!(
        "::notice title=Staged takanawa-node native artifacts::{}",
        package_dir.display()
    );
    Ok(())
}

fn verify_takanawa_node_npm_package() -> Result<()> {
    let package_dir = repo_root().join(TAKANAWA_NODE_PACKAGE_DIR);
    verify_takanawa_node_native_files(&package_dir)?;

    for relative_path in [
        "index.js",
        "dist/index.cjs",
        "dist/index.mjs",
        "dist/index.d.ts",
    ] {
        let path = package_dir.join(relative_path);
        if !path.is_file() {
            return Err(format!(
                "takanawa-node npm package is missing required file {}",
                path.display()
            )
            .into());
        }
    }

    Ok(())
}

fn verify_takanawa_node_native_files(package_dir: &Path) -> Result<()> {
    let mut actual_files = Vec::new();
    for entry in fs::read_dir(package_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("node") {
            continue;
        }
        actual_files.push(entry.file_name().to_string_lossy().into_owned());
    }
    actual_files.sort();

    let mut expected_files = TAKANAWA_NODE_NATIVE_FILES
        .iter()
        .map(|file_name| (*file_name).to_owned())
        .collect::<Vec<_>>();
    expected_files.sort();

    if actual_files != expected_files {
        return Err(format!(
            "takanawa-node native artifacts must be exactly {expected_files:?}, found {actual_files:?}"
        )
        .into());
    }

    Ok(())
}

fn prepare_capacitor_npm_package() -> Result<()> {
    let root = repo_root();
    let package_xcframework = root.join("packages/takanawa-capacitor/ios/Takanawa.xcframework");
    let package_takanawa_source = root.join("packages/takanawa-capacitor/ios/Sources/Takanawa");
    let swiftpm_zip = root.join("target/swiftpm/Takanawa.xcframework.zip");
    let local_xcframework = root.join("target/apple/Takanawa.xcframework");

    if package_xcframework.is_dir() {
        fs::remove_dir_all(&package_xcframework)?;
    }
    if package_takanawa_source.is_dir() {
        fs::remove_dir_all(&package_takanawa_source)?;
    }

    if swiftpm_zip.is_file() {
        run_command(
            repo_command("unzip")
                .args(["-q", "-o"])
                .arg(&swiftpm_zip)
                .arg("-d")
                .arg(root.join("packages/takanawa-capacitor/ios")),
        )?;
    } else if local_xcframework.is_dir() {
        copy_dir(&local_xcframework, &package_xcframework)?;
    } else {
        return Err(
            "missing Takanawa.xcframework for takanawa-capacitor; download the Apple artifact or run mise run package:apple first"
                .into(),
        );
    }

    if !package_xcframework.is_dir() {
        return Err("ios/Takanawa.xcframework was not staged for takanawa-capacitor".into());
    }
    verify_apple_xcframework(&package_xcframework)?;

    copy_dir(&root.join("Sources/Takanawa"), &package_takanawa_source)?;

    println!(
        "::notice title=Staged Capacitor XCFramework::{}",
        package_xcframework.display()
    );
    Ok(())
}
