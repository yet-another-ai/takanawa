use std::env;
use std::fs;

use crate::apple::stage_apple_framework;
use crate::release::workspace_version;
use crate::support::{
    Result, copy_dir, copy_file, deployment_env, ensure_dir, ensure_supported_windows_target,
    remove_dir_if_exists, repo_command, repo_root, run_command,
};

const GDEXTENSION_ADDON_SRC: &str = "packages/takanawa-gdextension/addons/takanawa";
const GDEXTENSION_STAGE: &str = "target/gdextension/addons/takanawa";
const GDEXTENSION_DIST: &str = "target/dist/takanawa-gdextension";

pub(crate) fn build_gdextension_desktop() -> Result<()> {
    if cfg!(target_os = "macos") {
        return build_gdextension_apple();
    }
    if cfg!(windows) {
        return build_gdextension_windows();
    }

    run_command(repo_command("cargo").args([
        "build",
        "-p",
        "takanawa-gdextension",
        "--release",
        "--locked",
    ]))?;
    ensure_gdextension_base()?;
    copy_file(
        "target/release/libtakanawa_gdextension.so",
        format!("{GDEXTENSION_STAGE}/bin/x86_64-unknown-linux-gnu/libtakanawa_gdextension.so"),
    )
}

pub(crate) fn build_gdextension_windows() -> Result<()> {
    let target = env::var("TAKANAWA_WINDOWS_TARGET")
        .map_err(|_| "TAKANAWA_WINDOWS_TARGET is required for build-gdextension-windows")?;
    ensure_supported_windows_target(&target)?;
    run_command(repo_command("rustup").args(["target", "add", target.as_str()]))?;
    run_command(repo_command("cargo").args([
        "build",
        "-p",
        "takanawa-gdextension",
        "--release",
        "--locked",
        "--target",
        target.as_str(),
    ]))?;
    ensure_gdextension_base()?;
    copy_file(
        format!("target/{target}/release/takanawa_gdextension.dll"),
        format!("{GDEXTENSION_STAGE}/bin/{target}/takanawa_gdextension.dll"),
    )
}

pub(crate) fn build_gdextension_android() -> Result<()> {
    run_command(repo_command("rustup").args([
        "target",
        "add",
        "aarch64-linux-android",
        "x86_64-linux-android",
    ]))?;
    remove_dir_if_exists("target/gdextension/android")?;
    run_command(repo_command("cargo").args([
        "ndk",
        "-t",
        "arm64-v8a",
        "-t",
        "x86_64",
        "--platform",
        "23",
        "-o",
        "target/gdextension/android",
        "build",
        "-p",
        "takanawa-gdextension",
        "--release",
        "--locked",
    ]))?;
    ensure_gdextension_base()?;
    copy_dir(
        &repo_root().join("target/gdextension/android"),
        &repo_root().join(format!("{GDEXTENSION_STAGE}/bin/android")),
    )
}

pub(crate) fn build_gdextension_apple() -> Result<()> {
    run_command(repo_command("rustup").args([
        "target",
        "add",
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "aarch64-apple-ios",
        "aarch64-apple-ios-sim",
        "x86_64-apple-ios",
    ]))?;

    for target in [
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "aarch64-apple-ios",
        "aarch64-apple-ios-sim",
        "x86_64-apple-ios",
    ] {
        run_command(deployment_env(repo_command("cargo")).args([
            "build",
            "-p",
            "takanawa-gdextension",
            "--release",
            "--locked",
            "--target",
            target,
        ]))?;
    }

    ensure_gdextension_base()?;
    let root = repo_root();
    let apple_dir = root.join("target/gdextension/apple");
    let macos_framework_dir = root.join(format!(
        "{GDEXTENSION_STAGE}/bin/universal-apple-darwin/Takanawa.framework"
    ));
    let ios_dir = root.join(format!("{GDEXTENSION_STAGE}/bin/ios"));
    fs::create_dir_all(&apple_dir)?;
    fs::create_dir_all(&ios_dir)?;

    let macos_dylib = apple_dir.join("Takanawa");
    run_command(
        repo_command("lipo")
            .args([
                "-create",
                "target/aarch64-apple-darwin/release/libtakanawa_gdextension.dylib",
                "target/x86_64-apple-darwin/release/libtakanawa_gdextension.dylib",
                "-output",
            ])
            .arg(&macos_dylib),
    )?;
    let version = workspace_version()?;
    stage_apple_framework(
        &macos_framework_dir,
        &macos_dylib,
        "Takanawa",
        &version,
        "10.15",
    )?;

    let device_framework = apple_dir.join("ios-arm64/Takanawa.framework");
    stage_apple_framework(
        &device_framework,
        &root.join("target/aarch64-apple-ios/release/libtakanawa_gdextension.dylib"),
        "Takanawa",
        &version,
        "12.0",
    )?;

    let simulator_dylib = apple_dir.join("Takanawa-ios-simulator");
    run_command(
        repo_command("lipo")
            .args([
                "-create",
                "target/aarch64-apple-ios-sim/release/libtakanawa_gdextension.dylib",
                "target/x86_64-apple-ios/release/libtakanawa_gdextension.dylib",
                "-output",
            ])
            .arg(&simulator_dylib),
    )?;
    let simulator_framework = apple_dir.join("ios-arm64_x86_64-simulator/Takanawa.framework");
    stage_apple_framework(
        &simulator_framework,
        &simulator_dylib,
        "Takanawa",
        &version,
        "12.0",
    )?;

    let xcframework = ios_dir.join("TakanawaGDExtension.xcframework");
    if xcframework.is_dir() {
        fs::remove_dir_all(&xcframework)?;
    }
    run_command(
        repo_command("xcodebuild")
            .args(["-create-xcframework", "-framework"])
            .arg(&device_framework)
            .arg("-framework")
            .arg(&simulator_framework)
            .args(["-output"])
            .arg(&xcframework),
    )
}

fn ensure_gdextension_base() -> Result<()> {
    ensure_dir(GDEXTENSION_STAGE)?;
    copy_file(
        format!("{GDEXTENSION_ADDON_SRC}/takanawa.gdextension"),
        format!("{GDEXTENSION_STAGE}/takanawa.gdextension"),
    )?;
    copy_file(
        format!("{GDEXTENSION_ADDON_SRC}/takanawa.gdextension.uid"),
        format!("{GDEXTENSION_STAGE}/takanawa.gdextension.uid"),
    )
}

pub(crate) fn dist_gdextension() -> Result<()> {
    ensure_gdextension_base()?;
    remove_dir_if_exists(GDEXTENSION_DIST)?;
    ensure_dir("target/dist")?;
    copy_dir(
        &repo_root().join("target/gdextension/addons"),
        &repo_root().join(format!("{GDEXTENSION_DIST}/addons")),
    )?;

    let zip_path = repo_root().join("target/dist/takanawa-gdextension.zip");
    if zip_path.is_file() {
        fs::remove_file(&zip_path)?;
    }
    if cfg!(windows) {
        let command = format!(
            "Compress-Archive -Path '{}/*' -DestinationPath '{}' -Force",
            repo_root().join(GDEXTENSION_DIST).display(),
            zip_path.display()
        );
        run_command(repo_command("powershell").args(["-NoProfile", "-Command", command.as_str()]))?;
    } else {
        run_command(
            repo_command("zip")
                .current_dir(repo_root().join(GDEXTENSION_DIST))
                .args(["-r", "../takanawa-gdextension.zip", "addons"]),
        )?;
    }
    Ok(())
}
