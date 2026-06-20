use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::release::workspace_version;
use crate::support::{
    Result, TestHttpServer, output_text, prepend_dynamic_library_path, repo_command, repo_root,
    run_command,
};

const NUGET_IOS_XCFRAMEWORK_INFO_PLIST: &str =
    "runtimes/ios/native/Takanawa.xcframework/Info.plist";
const NUGET_IOS_DEVICE_XCFRAMEWORK_LIBRARY: &str =
    "runtimes/ios/native/Takanawa.xcframework/ios-arm64/libtakanawa_ffi.a";
const NUGET_IOS_SIMULATOR_XCFRAMEWORK_LIBRARY: &str =
    "runtimes/ios/native/Takanawa.xcframework/ios-arm64_x86_64-simulator/libtakanawa_ffi.a";
const NUGET_MACOS_XCFRAMEWORK_LIBRARY: &str =
    "runtimes/ios/native/Takanawa.xcframework/macos-arm64_x86_64/libtakanawa_ffi.a";

pub(crate) fn check_csharp() -> Result<()> {
    let native_dir = repo_root().join("target/debug");
    run_command(repo_command("dotnet").args([
        "build",
        "packages/takanawa-csharp/src/YetAnotherAI.Takanawa/YetAnotherAI.Takanawa.csproj",
        "--configuration",
        "Release",
    ]))?;
    run_command(repo_command("dotnet").args([
        "test",
        "packages/takanawa-csharp/tests/YetAnotherAI.Takanawa.Tests/YetAnotherAI.Takanawa.Tests.csproj",
        "--configuration",
        "Release",
    ]))?;
    run_command(repo_command("cargo").args(["build", "-p", "takanawa-ffi", "--locked"]))?;

    let server = TestHttpServer::start()?;
    let mut smoke = repo_command("dotnet");
    smoke.args([
        "run",
        "--project",
        "fixtures/csharp-smoke/YetAnotherAI.Takanawa.Smoke.csproj",
        "--configuration",
        "Release",
    ]);
    prepend_dynamic_library_path(&mut smoke, &native_dir);
    server.configure_command(&mut smoke);
    run_command(&mut smoke)?;

    let version = workspace_version()?;
    let output_dir = repo_root().join("target/csharp/check-package");
    fs::create_dir_all(&output_dir)?;
    let output_dir = output_dir
        .to_str()
        .ok_or("target/csharp/check-package is not valid UTF-8")?
        .to_owned();
    let version_property = format!("-p:Version={version}");
    run_command(repo_command("dotnet").args([
        "pack",
        "packages/takanawa-csharp/src/YetAnotherAI.Takanawa/YetAnotherAI.Takanawa.csproj",
        "--configuration",
        "Release",
        "--output",
        output_dir.as_str(),
        "-p:ContinuousIntegrationBuild=true",
        version_property.as_str(),
    ]))
}

pub(crate) fn pack_csharp() -> Result<()> {
    let version = workspace_version()?;
    let package_path = pack_csharp_version(&version, "target/csharp/package")?;
    verify_csharp_package(&package_path)
}

fn pack_csharp_version(version: &str, output_dir: impl AsRef<Path>) -> Result<PathBuf> {
    let output_dir = repo_root().join(output_dir);
    fs::create_dir_all(&output_dir)?;
    let output_dir_text = output_dir
        .to_str()
        .ok_or("C# package output directory is not valid UTF-8")?
        .to_owned();
    let version_property = format!("-p:Version={version}");
    run_command(repo_command("dotnet").args([
        "pack",
        "packages/takanawa-csharp/src/YetAnotherAI.Takanawa/YetAnotherAI.Takanawa.csproj",
        "--configuration",
        "Release",
        "--output",
        output_dir_text.as_str(),
        "-p:ContinuousIntegrationBuild=true",
        version_property.as_str(),
    ]))?;
    Ok(output_dir.join(format!("YetAnotherAI.Takanawa.{version}.nupkg")))
}

fn verify_csharp_package(package_path: &Path) -> Result<()> {
    if !package_path.is_file() {
        return Err(format!("missing NuGet package {}", package_path.display()).into());
    }

    let listing = output_text(
        repo_command("unzip").args([
            "-Z1",
            package_path
                .to_str()
                .ok_or("NuGet package path is not valid UTF-8")?,
        ]),
    )?;
    let entries = listing.lines().collect::<Vec<_>>();
    for required in required_csharp_package_entries() {
        if !entries.contains(required) {
            return Err(format!(
                "{} is missing required package entry {required}",
                package_path.display()
            )
            .into());
        }
    }
    verify_csharp_package_ios_xcframework(package_path)?;

    Ok(())
}

fn required_csharp_package_entries() -> &'static [&'static str] {
    &[
        "README.md",
        "lib/netstandard2.0/YetAnotherAI.Takanawa.dll",
        "lib/netstandard2.0/YetAnotherAI.Takanawa.xml",
        "buildTransitive/YetAnotherAI.Takanawa.targets",
        "runtimes/win-x64/native/takanawa_ffi.dll",
        "runtimes/win-arm64/native/takanawa_ffi.dll",
        "runtimes/linux-x64/native/libtakanawa_ffi.so",
        "runtimes/osx-x64/native/libtakanawa_ffi.dylib",
        "runtimes/osx-arm64/native/libtakanawa_ffi.dylib",
        "runtimes/android-arm64/native/libtakanawa_ffi.so",
        "runtimes/android-x64/native/libtakanawa_ffi.so",
        NUGET_IOS_XCFRAMEWORK_INFO_PLIST,
        NUGET_IOS_DEVICE_XCFRAMEWORK_LIBRARY,
        NUGET_IOS_SIMULATOR_XCFRAMEWORK_LIBRARY,
        NUGET_MACOS_XCFRAMEWORK_LIBRARY,
    ]
}

fn verify_csharp_package_ios_xcframework(package_path: &Path) -> Result<()> {
    let info_plist = output_text(
        repo_command("unzip")
            .arg("-p")
            .arg(package_path)
            .arg(NUGET_IOS_XCFRAMEWORK_INFO_PLIST),
    )?;
    for required in [
        "ios-arm64",
        "ios-arm64_x86_64-simulator",
        "macos-arm64_x86_64",
        "SupportedArchitectures",
        "<string>arm64</string>",
        "<string>x86_64</string>",
        "<string>simulator</string>",
    ] {
        if !info_plist.contains(required) {
            return Err(format!(
                "{} is missing required iOS XCFramework metadata {required}",
                package_path.display()
            )
            .into());
        }
    }

    Ok(())
}

pub(crate) fn prepare_csharp_nuget_assets() -> Result<()> {
    let artifacts_dir = repo_root().join("target/release-artifacts");
    if !artifacts_dir.is_dir() {
        return Err(format!(
            "missing release artifacts directory {}",
            artifacts_dir.display()
        )
        .into());
    }

    fs::create_dir_all(repo_root().join("target/dist"))?;
    fs::create_dir_all(repo_root().join("target/apple"))?;

    for archive in [
        "takanawa-linux-x86_64.tar.gz",
        "takanawa-macos-universal.tar.gz",
        "takanawa-android-jniLibs.tar.gz",
    ] {
        let path = artifacts_dir.join(archive);
        if path.is_file() {
            run_command(
                repo_command("tar")
                    .args(["-C", "target/dist", "-xzf"])
                    .arg(path),
            )?;
        }
    }

    for archive in [
        "takanawa-windows-x86_64.zip",
        "takanawa-windows-aarch64.zip",
    ] {
        let path = artifacts_dir.join(archive);
        if path.is_file() {
            run_command(
                repo_command("unzip")
                    .arg("-q")
                    .arg("-o")
                    .arg(path)
                    .arg("-d")
                    .arg("target/dist"),
            )?;
        }
    }

    let xcframework_zip = artifacts_dir.join("Takanawa.xcframework.zip");
    if xcframework_zip.is_file() {
        run_command(
            repo_command("unzip")
                .arg("-q")
                .arg("-o")
                .arg(xcframework_zip)
                .arg("-d")
                .arg("target/apple"),
        )?;
    }

    Ok(())
}

pub(crate) fn nuget_publish(mode: &str) -> Result<()> {
    if mode != "dry-run" && mode != "publish" {
        return Err("usage: xtask nuget-publish <dry-run|publish>".into());
    }

    pack_csharp()?;
    if mode == "dry-run" {
        return Ok(());
    }

    let api_key = env::var("NUGET_API_KEY")
        .map_err(|_| "NUGET_API_KEY is required; use NuGet/login@v1 with OIDC to issue it")?;
    if api_key.is_empty() {
        return Err("NUGET_API_KEY is required; use NuGet/login@v1 with OIDC to issue it".into());
    }

    let version = workspace_version()?;
    let package_path = repo_root()
        .join("target/csharp/package")
        .join(format!("YetAnotherAI.Takanawa.{version}.nupkg"));
    run_command(
        repo_command("dotnet").args([
            "nuget",
            "push",
            package_path
                .to_str()
                .ok_or("NuGet package path is not valid UTF-8")?,
            "--api-key",
            api_key.as_str(),
            "--source",
            "https://api.nuget.org/v3/index.json",
            "--skip-duplicate",
        ]),
    )
}
