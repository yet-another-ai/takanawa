use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::support::{
    Result, TestHttpServer, copy_dir, copy_file, copy_file_if_exists, deployment_env, ensure_dir,
    ensure_supported_windows_target, prepend_dynamic_library_path, remove_dir_if_exists,
    repo_command, repo_root, run_command,
};

pub(crate) fn cargo_check() -> Result<()> {
    run_command(repo_command("cargo").args([
        "check",
        "--workspace",
        "--all-features",
        "--locked",
    ]))?;
    ensure_dir("target/debug")?;
    fs::write(repo_root().join("target/debug/.cargo-check-stamp"), "")?;
    Ok(())
}

pub(crate) fn generate_header() -> Result<()> {
    run_command(repo_command("cbindgen").args([
        "--config",
        "cbindgen.toml",
        "--crate",
        "takanawa-ffi",
        "--output",
        "include/takanawa.h",
    ]))
}

pub(crate) fn build_windows_ffi() -> Result<()> {
    let target = env::var("TAKANAWA_WINDOWS_TARGET")
        .map_err(|_| "TAKANAWA_WINDOWS_TARGET is required for build-windows-ffi")?;
    ensure_supported_windows_target(&target)?;
    generate_header()?;
    run_command(repo_command("rustup").args(["target", "add", target.as_str()]))?;
    run_command(repo_command("cargo").args([
        "build",
        "-p",
        "takanawa-ffi",
        "--release",
        "--locked",
        "--target",
        target.as_str(),
    ]))
}

pub(crate) fn dist_windows() -> Result<()> {
    let target = env::var("TAKANAWA_WINDOWS_TARGET")
        .map_err(|_| "TAKANAWA_WINDOWS_TARGET is required for dist-windows")?;
    let artifact = env::var("TAKANAWA_WINDOWS_ARTIFACT")
        .map_err(|_| "TAKANAWA_WINDOWS_ARTIFACT is required for dist-windows")?;

    build_windows_ffi()?;

    let artifact_dir = repo_root().join("target/dist").join(&artifact);
    fs::create_dir_all(&artifact_dir)?;
    copy_file(
        format!("target/{target}/release/takanawa_ffi.dll"),
        artifact_dir.join("takanawa_ffi.dll"),
    )?;
    copy_file_if_exists(
        format!("target/{target}/release/takanawa_ffi.dll.lib"),
        artifact_dir.join("takanawa_ffi.dll.lib"),
    )?;
    copy_file_if_exists(
        format!("target/{target}/release/takanawa_ffi.lib"),
        artifact_dir.join("takanawa_ffi.lib"),
    )?;
    copy_file("include/takanawa.h", artifact_dir.join("takanawa.h"))?;

    let zip_path = repo_root()
        .join("target/dist")
        .join(format!("{artifact}.zip"));
    if cfg!(windows) {
        let command = format!(
            "Compress-Archive -Path '{}' -DestinationPath '{}' -Force",
            artifact_dir.display(),
            zip_path.display()
        );
        run_command(repo_command("powershell").args(["-NoProfile", "-Command", command.as_str()]))?;
    } else {
        run_command(
            repo_command("zip")
                .arg("-r")
                .arg(zip_path)
                .arg(artifact_dir),
        )?;
    }

    Ok(())
}

pub(crate) fn dist_linux() -> Result<()> {
    generate_header()?;
    run_command(repo_command("cargo").args([
        "build",
        "-p",
        "takanawa-ffi",
        "--release",
        "--locked",
    ]))?;
    ensure_dir("target/dist/takanawa-linux-x86_64")?;
    copy_file(
        "target/release/libtakanawa_ffi.so",
        "target/dist/takanawa-linux-x86_64/libtakanawa_ffi.so",
    )?;
    copy_file(
        "include/takanawa.h",
        "target/dist/takanawa-linux-x86_64/takanawa.h",
    )?;
    run_command(repo_command("tar").args([
        "-C",
        "target/dist",
        "-czf",
        "target/dist/takanawa-linux-x86_64.tar.gz",
        "takanawa-linux-x86_64",
    ]))
}

pub(crate) fn dist_macos_universal() -> Result<()> {
    generate_header()?;
    run_command(repo_command("rustup").args([
        "target",
        "add",
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
    ]))?;
    for target in ["aarch64-apple-darwin", "x86_64-apple-darwin"] {
        run_command(deployment_env(repo_command("cargo")).args([
            "build",
            "-p",
            "takanawa-ffi",
            "--release",
            "--locked",
            "--target",
            target,
        ]))?;
    }
    ensure_dir("target/dist/takanawa-macos-universal")?;
    run_command(repo_command("lipo").args([
        "-create",
        "target/aarch64-apple-darwin/release/libtakanawa_ffi.dylib",
        "target/x86_64-apple-darwin/release/libtakanawa_ffi.dylib",
        "-output",
        "target/dist/takanawa-macos-universal/libtakanawa_ffi.dylib",
    ]))?;
    copy_file(
        "include/takanawa.h",
        "target/dist/takanawa-macos-universal/takanawa.h",
    )?;
    run_command(repo_command("tar").args([
        "-C",
        "target/dist",
        "-czf",
        "target/dist/takanawa-macos-universal.tar.gz",
        "takanawa-macos-universal",
    ]))
}

pub(crate) fn dist_android() -> Result<()> {
    ensure_dir("target/dist/takanawa-android")?;
    remove_dir_if_exists("target/dist/takanawa-android/jniLibs")?;
    copy_dir(
        &repo_root().join("target/android/jniLibs"),
        &repo_root().join("target/dist/takanawa-android/jniLibs"),
    )?;
    copy_file(
        "include/takanawa.h",
        "target/dist/takanawa-android/takanawa.h",
    )?;
    copy_file(
        "android/takanawa-android/build/outputs/aar/takanawa-android-release.aar",
        "target/dist/takanawa-android-release.aar",
    )?;
    run_command(repo_command("tar").args([
        "-C",
        "target/dist",
        "-czf",
        "target/dist/takanawa-android-jniLibs.tar.gz",
        "takanawa-android",
    ]))
}

pub(crate) fn test_cmake_integration() -> Result<()> {
    let build_dir = env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("takanawa-cmake-integration");
    if build_dir.is_dir() {
        fs::remove_dir_all(&build_dir)?;
    }

    run_command(
        repo_command("cmake")
            .args(["-S", "fixtures/cmake-integration", "-B"])
            .arg(&build_dir)
            .arg("-DTAKANAWA_CARGO_PROFILE=debug"),
    )?;
    run_command(repo_command("cmake").arg("--build").arg(&build_dir))?;
    let server = TestHttpServer::start()?;
    let mut smoke = Command::new(build_dir.join("takanawa_cpp_smoke"));
    smoke.current_dir(&build_dir);
    prepend_dynamic_library_path(&mut smoke, &build_dir.join("takanawa-build/cargo/debug"));
    server.configure_command(&mut smoke);
    run_command(&mut smoke)
}
