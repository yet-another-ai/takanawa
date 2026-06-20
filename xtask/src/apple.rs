use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::release::workspace_version;
use crate::support::{
    Result, TestHttpServer, copy_dir, copy_file, deployment_env, ensure_dir, output_text,
    remove_dir_if_exists, repo_command, repo_root, run_command,
};

const IOS_XCFRAMEWORK_INFO_PLIST: &str = "Takanawa.xcframework/Info.plist";
const IOS_DEVICE_XCFRAMEWORK_LIBRARY: &str = "Takanawa.xcframework/ios-arm64/libtakanawa_ffi.a";
const IOS_SIMULATOR_XCFRAMEWORK_LIBRARY: &str =
    "Takanawa.xcframework/ios-arm64_x86_64-simulator/libtakanawa_ffi.a";
const MACOS_XCFRAMEWORK_LIBRARY: &str = "Takanawa.xcframework/macos-arm64_x86_64/libtakanawa_ffi.a";

pub(crate) fn build_apple_xcframework() -> Result<()> {
    run_command(repo_command("rustup").args([
        "target",
        "add",
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "aarch64-apple-ios",
        "aarch64-apple-ios-sim",
        "x86_64-apple-ios",
    ]))?;

    let targets = [
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "aarch64-apple-ios",
        "aarch64-apple-ios-sim",
        "x86_64-apple-ios",
    ];
    for target in targets {
        run_command(deployment_env(repo_command("cargo")).args([
            "build",
            "-p",
            "takanawa-ffi",
            "--release",
            "--target",
            target,
        ]))?;
    }

    ensure_dir("target/apple/macos")?;
    ensure_dir("target/apple/ios-simulator")?;
    run_command(repo_command("lipo").args([
        "-create",
        "target/aarch64-apple-darwin/release/libtakanawa_ffi.a",
        "target/x86_64-apple-darwin/release/libtakanawa_ffi.a",
        "-output",
        "target/apple/macos/libtakanawa_ffi.a",
    ]))?;
    run_command(repo_command("lipo").args([
        "-create",
        "target/aarch64-apple-ios-sim/release/libtakanawa_ffi.a",
        "target/x86_64-apple-ios/release/libtakanawa_ffi.a",
        "-output",
        "target/apple/ios-simulator/libtakanawa_ffi.a",
    ]))?;

    remove_dir_if_exists("target/apple/Takanawa.xcframework")?;
    run_command(repo_command("xcodebuild").args([
        "-create-xcframework",
        "-library",
        "target/apple/macos/libtakanawa_ffi.a",
        "-headers",
        "include",
        "-library",
        "target/aarch64-apple-ios/release/libtakanawa_ffi.a",
        "-headers",
        "include",
        "-library",
        "target/apple/ios-simulator/libtakanawa_ffi.a",
        "-headers",
        "include",
        "-output",
        "target/apple/Takanawa.xcframework",
    ]))?;

    verify_apple_xcframework(&repo_root().join("target/apple/Takanawa.xcframework"))?;
    package_swiftpm()
}

pub(crate) fn verify_apple_xcframework(xcframework: &Path) -> Result<()> {
    for relative in [
        IOS_XCFRAMEWORK_INFO_PLIST,
        IOS_DEVICE_XCFRAMEWORK_LIBRARY,
        IOS_SIMULATOR_XCFRAMEWORK_LIBRARY,
        MACOS_XCFRAMEWORK_LIBRARY,
    ] {
        let relative_path = relative
            .strip_prefix("Takanawa.xcframework/")
            .unwrap_or(relative);
        let path = xcframework.join(relative_path);
        if !path.is_file() {
            return Err(format!(
                "{} is missing expected XCFramework entry {relative_path}",
                xcframework.display()
            )
            .into());
        }
    }

    let info_plist = fs::read_to_string(
        xcframework.join(
            IOS_XCFRAMEWORK_INFO_PLIST
                .strip_prefix("Takanawa.xcframework/")
                .expect("Info.plist path should be relative to Takanawa.xcframework"),
        ),
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
                "{} is missing expected XCFramework metadata {required}",
                xcframework.display()
            )
            .into());
        }
    }

    Ok(())
}

pub(crate) fn stage_apple_framework(
    framework_dir: &Path,
    source_binary: &Path,
    executable: &str,
    version: &str,
    minimum_os_version: &str,
) -> Result<()> {
    if framework_dir.is_dir() {
        fs::remove_dir_all(framework_dir)?;
    }
    fs::create_dir_all(framework_dir)?;

    let framework_binary = framework_dir.join(executable);
    fs::copy(source_binary, &framework_binary)?;
    fs::write(
        framework_dir.join("Info.plist"),
        framework_info_plist(executable, version, minimum_os_version),
    )?;

    let install_name = format!("@rpath/{executable}.framework/{executable}");
    run_command(
        repo_command("install_name_tool")
            .args(["-id", install_name.as_str()])
            .arg(&framework_binary),
    )
}

fn framework_info_plist(executable: &str, version: &str, minimum_os_version: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>
  <string>{executable}</string>
  <key>CFBundleIdentifier</key>
  <string>ai.yetanother.takanawa.gdextension</string>
  <key>CFBundleName</key>
  <string>{executable}</string>
  <key>CFBundlePackageType</key>
  <string>FMWK</string>
  <key>CFBundleVersion</key>
  <string>{version}</string>
  <key>MinimumOSVersion</key>
  <string>{minimum_os_version}</string>
</dict>
</plist>
"#
    )
}

pub(crate) fn package_swiftpm() -> Result<()> {
    let root = repo_root();
    let xcframework = root.join("target/apple/Takanawa.xcframework");
    if !xcframework.is_dir() {
        return Err(
            "missing target/apple/Takanawa.xcframework; run mise run package:apple first".into(),
        );
    }

    let package_dir = root.join("target/swiftpm");
    let zip_path = package_dir.join("Takanawa.xcframework.zip");
    let checksum_path = package_dir.join("Takanawa.xcframework.zip.checksum");
    let staging_dir = package_dir.join("staging");

    fs::create_dir_all(&package_dir)?;
    if zip_path.is_file() {
        fs::remove_file(&zip_path)?;
    }
    if checksum_path.is_file() {
        fs::remove_file(&checksum_path)?;
    }
    if staging_dir.is_dir() {
        fs::remove_dir_all(&staging_dir)?;
    }
    fs::create_dir_all(&staging_dir)?;

    run_command(repo_command("ditto").args([
        xcframework.as_os_str(),
        staging_dir.join("Takanawa.xcframework").as_os_str(),
    ]))?;

    let mut entries = collect_relative_entries(&staging_dir, Path::new("Takanawa.xcframework"))?;
    entries.sort();
    for entry in &entries {
        run_command(
            repo_command("touch")
                .args(["-h", "-t", "202001010000.00"])
                .arg(staging_dir.join(entry)),
        )?;
    }

    let mut zip = Command::new("zip")
        .current_dir(&staging_dir)
        .args(["-X", "-q", "-@", "../Takanawa.xcframework.zip"])
        .stdin(Stdio::piped())
        .spawn()?;
    {
        let stdin = zip.stdin.as_mut().expect("zip stdin should be piped");
        for entry in &entries {
            writeln!(stdin, "{}", entry.to_string_lossy().replace('\\', "/"))?;
        }
    }
    let status = zip.wait()?;
    if !status.success() {
        return Err(format!("zip exited with {status}").into());
    }

    let checksum = output_text(
        repo_command("swift")
            .args(["package", "compute-checksum"])
            .arg(&zip_path),
    )?;
    fs::write(&checksum_path, format!("{checksum}\n"))?;

    println!("Created {}", zip_path.display());
    println!("SwiftPM checksum: {checksum}");
    Ok(())
}

fn collect_relative_entries(base: &Path, relative: &Path) -> Result<Vec<PathBuf>> {
    let path = base.join(relative);
    let mut entries = vec![relative.to_path_buf()];
    if path.is_dir() {
        let mut children = Vec::new();
        for child in fs::read_dir(path)? {
            children.push(child?.file_name());
        }
        children.sort();
        for child in children {
            entries.extend(collect_relative_entries(base, &relative.join(child))?);
        }
    }
    Ok(entries)
}

pub(crate) fn dist_apple_swiftpm() -> Result<()> {
    ensure_dir("target/dist")?;
    copy_file(
        "target/swiftpm/Takanawa.xcframework.zip",
        "target/dist/Takanawa.xcframework.zip",
    )?;
    copy_file(
        "target/swiftpm/Takanawa.xcframework.zip.checksum",
        "target/dist/Takanawa.xcframework.zip.checksum",
    )?;
    copy_file("target/swiftpm/Package.swift", "target/dist/Package.swift")?;
    Ok(())
}

pub(crate) fn test_capacitor_ios() -> Result<()> {
    if !repo_root()
        .join("target/apple/Takanawa.xcframework")
        .is_dir()
    {
        return Err(
            "missing target/apple/Takanawa.xcframework; run mise run package:apple first".into(),
        );
    }

    let sdk_path =
        output_text(repo_command("xcrun").args(["--sdk", "iphonesimulator", "--show-sdk-path"]))?;
    let arch = output_text(repo_command("uname").arg("-m"))?;
    let triple = match arch.as_str() {
        "arm64" | "aarch64" => "arm64-apple-ios-simulator",
        "x86_64" => "x86_64-apple-ios-simulator",
        _ => {
            return Err(
                format!("unsupported host architecture for iOS simulator build: {arch}").into(),
            );
        }
    };

    run_command(
        repo_command("swift")
            .env("TAKANAWA_CAPACITOR_USE_LOCAL_TAKANAWA", "1")
            .args([
                "build",
                "--package-path",
                "packages/takanawa-capacitor",
                "--triple",
                triple,
                "-Xswiftc",
                "-sdk",
                "-Xswiftc",
                sdk_path.as_str(),
                "-Xcc",
                "-isysroot",
                "-Xcc",
                sdk_path.as_str(),
            ]),
    )
}

pub(crate) fn test_swift_integration() -> Result<()> {
    let root = repo_root();
    let zip_path = env::var_os("TAKANAWA_XCFRAMEWORK_ZIP")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target/swiftpm/Takanawa.xcframework.zip"));
    let xcframework_path = env::var_os("TAKANAWA_XCFRAMEWORK_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target/apple/Takanawa.xcframework"));
    let work_dir = root.join("target/swift-integration");
    let package_dir = work_dir.join("package");

    if work_dir.is_dir() {
        fs::remove_dir_all(&work_dir)?;
    }
    fs::create_dir_all(&work_dir)?;
    copy_dir(&root.join("fixtures/swift-integration"), &package_dir)?;
    let source_target = package_dir.join("Sources/Takanawa");
    if source_target.is_dir() {
        fs::remove_dir_all(&source_target)?;
    }
    copy_dir(&root.join("Sources/Takanawa"), &source_target)?;

    if zip_path.is_file() {
        run_command(
            repo_command("unzip")
                .arg("-q")
                .arg(&zip_path)
                .arg("-d")
                .arg(&package_dir),
        )?;
    } else if xcframework_path.is_dir() {
        run_command(
            repo_command("ditto")
                .arg(&xcframework_path)
                .arg(package_dir.join("Takanawa.xcframework")),
        )?;
    } else {
        return Err("missing SwiftPM zip or XCFramework; run mise run package:apple first".into());
    }

    if !package_dir.join("Takanawa.xcframework").is_dir() {
        return Err("Takanawa.xcframework was not found in the Swift integration package".into());
    }

    run_command(
        repo_command("swift")
            .args(["build", "--package-path"])
            .arg(&package_dir),
    )?;
    let server = TestHttpServer::start()?;
    let mut smoke = repo_command("swift");
    smoke
        .args(["run", "--package-path"])
        .arg(&package_dir)
        .arg("TakanawaSmoke");
    server.configure_command(&mut smoke);
    run_command(&mut smoke)
}

pub(crate) fn check_apple() -> Result<()> {
    test_capacitor_ios()?;
    test_swift_integration()
}

pub(crate) fn swiftpm_release_manifest(version_arg: Option<String>) -> Result<()> {
    let mut version = version_arg.unwrap_or_default();
    if version.is_empty() {
        version = workspace_version()?;
    }
    let version = version.strip_prefix('v').unwrap_or(&version).to_owned();
    if version.is_empty() {
        return Err("missing release version".into());
    }

    let checksum_path = repo_root().join("target/swiftpm/Takanawa.xcframework.zip.checksum");
    if !checksum_path.is_file() {
        return Err("missing target/swiftpm/Takanawa.xcframework.zip.checksum; run mise run package:swiftpm first".into());
    }
    let checksum = fs::read_to_string(checksum_path)?.trim().to_owned();
    let output_path = env::var_os("TAKANAWA_SWIFTPM_RELEASE_MANIFEST")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root().join("target/swiftpm/Package.swift"));
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(
        &output_path,
        format!(
            r#"// swift-tools-version: 5.9
import PackageDescription

let package = Package(
  name: "Takanawa",
  platforms: [
    .iOS(.v13),
    .macOS(.v10_15)
  ],
  products: [
    .library(
      name: "Takanawa",
      targets: ["Takanawa"]
    )
  ],
  targets: [
    .target(
      name: "Takanawa",
      dependencies: ["TakanawaBinary"],
      linkerSettings: [
        .linkedFramework("CoreFoundation"),
        .linkedFramework("Security"),
        .linkedLibrary("iconv")
      ]
    ),
    .binaryTarget(
      name: "TakanawaBinary",
      url: "https://github.com/yet-another-ai/takanawa/releases/download/v{version}/Takanawa.xcframework.zip",
      checksum: "{checksum}"
    )
  ]
)
"#
        ),
    )?;

    println!("Generated {} for v{version}", output_path.display());
    Ok(())
}
