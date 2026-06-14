use std::ffi::OsStr;
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{env, thread, time};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const TEST_HTTP_PAYLOAD: &[u8] = b"takanawa integration fixture payload\n";
const IOS_XCFRAMEWORK_INFO_PLIST: &str = "Takanawa.xcframework/Info.plist";
const IOS_DEVICE_XCFRAMEWORK_LIBRARY: &str = "Takanawa.xcframework/ios-arm64/libtakanawa_ffi.a";
const IOS_SIMULATOR_XCFRAMEWORK_LIBRARY: &str =
    "Takanawa.xcframework/ios-arm64_x86_64-simulator/libtakanawa_ffi.a";
const MACOS_XCFRAMEWORK_LIBRARY: &str = "Takanawa.xcframework/macos-arm64_x86_64/libtakanawa_ffi.a";
const NUGET_IOS_XCFRAMEWORK_INFO_PLIST: &str =
    "runtimes/ios/native/Takanawa.xcframework/Info.plist";
const NUGET_IOS_DEVICE_XCFRAMEWORK_LIBRARY: &str =
    "runtimes/ios/native/Takanawa.xcframework/ios-arm64/libtakanawa_ffi.a";
const NUGET_IOS_SIMULATOR_XCFRAMEWORK_LIBRARY: &str =
    "runtimes/ios/native/Takanawa.xcframework/ios-arm64_x86_64-simulator/libtakanawa_ffi.a";
const NUGET_MACOS_XCFRAMEWORK_LIBRARY: &str =
    "runtimes/ios/native/Takanawa.xcframework/macos-arm64_x86_64/libtakanawa_ffi.a";

fn main() {
    if let Err(error) = run_main() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run_main() -> Result<()> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print_usage();
        return Err("missing xtask command".into());
    };

    match command.as_str() {
        "android-sdk" => android_sdk(),
        "build-android" => build_android(),
        "build-apple-xcframework" => build_apple_xcframework(),
        "build-windows-ffi" => build_windows_ffi(),
        "cargo-check" => cargo_check(),
        "check-apple" => check_apple(),
        "check-csharp" => check_csharp(),
        "dist-android" => dist_android(),
        "dist-apple-swiftpm" => dist_apple_swiftpm(),
        "dist-linux" => dist_linux(),
        "dist-macos-universal" => dist_macos_universal(),
        "dist-windows" => dist_windows(),
        "github-release" => github_release(),
        "npm-publish" => npm_publish(args.next().as_deref().unwrap_or("")),
        "nuget-publish" => nuget_publish(args.next().as_deref().unwrap_or("")),
        "package-swiftpm" => package_swiftpm(),
        "pack-csharp" => pack_csharp(),
        "prepare-csharp-nuget-assets" => prepare_csharp_nuget_assets(),
        "publish-android-local" => publish_android_local(),
        "publish-android-central" => publish_android_central(),
        "publish-crates" => publish_crates(),
        "swiftpm-release-manifest" => {
            swiftpm_release_manifest(args.next().or_else(|| env::var("GITHUB_REF_NAME").ok()))
        }
        "sync-version" => {
            let version = args.next();
            if args.next().is_some() {
                return Err("usage: xtask sync-version [version]".into());
            }
            sync_version(version)
        }
        "test-capacitor-ios" => test_capacitor_ios(),
        "test-cmake-integration" => test_cmake_integration(),
        "test-swift-integration" => test_swift_integration(),
        "validate-maven-central-env" => validate_maven_central_env(),
        _ => {
            print_usage();
            Err(format!("unknown xtask command: {command}").into())
        }
    }
}

fn print_usage() {
    eprintln!(
        "usage: cargo run -p xtask -- <command>\n\n\
         commands:\n  \
         android-sdk\n  \
         build-android\n  \
         build-apple-xcframework\n  \
         build-windows-ffi\n  \
         cargo-check\n  \
         check-apple\n  \
         check-csharp\n  \
         dist-android\n  \
         dist-apple-swiftpm\n  \
         dist-linux\n  \
         dist-macos-universal\n  \
         dist-windows\n  \
         github-release\n  \
         npm-publish <dry-run|publish>\n  \
         nuget-publish <dry-run|publish>\n  \
         package-swiftpm\n  \
         pack-csharp\n  \
         prepare-csharp-nuget-assets\n  \
         publish-android-local\n  \
         publish-android-central\n  \
         publish-crates\n  \
         swiftpm-release-manifest [version]\n  \
         sync-version [version]\n  \
         test-capacitor-ios\n  \
         test-cmake-integration\n  \
         test-swift-integration\n  \
         validate-maven-central-env"
    );
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask manifest should live under repo root")
        .to_path_buf()
}

fn repo_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    command.current_dir(repo_root());
    command
}

struct TestHttpServer {
    addr: SocketAddr,
    shutdown: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl TestHttpServer {
    fn start() -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let addr = listener.local_addr()?;
        let payload = Arc::new(TEST_HTTP_PAYLOAD.to_vec());
        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_shutdown = Arc::clone(&shutdown);
        let thread = thread::spawn(move || {
            while !thread_shutdown.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let payload = Arc::clone(&payload);
                        thread::spawn(move || handle_test_http_connection(stream, &payload));
                    }
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {
                        thread::sleep(time::Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            addr,
            shutdown,
            thread: Some(thread),
        })
    }

    fn url(&self) -> String {
        format!("http://{}/file", self.addr)
    }

    fn configure_command(&self, command: &mut Command) {
        command.env("TAKANAWA_TEST_URL", self.url());
        command.env(
            "TAKANAWA_TEST_EXPECTED_BYTES",
            String::from_utf8_lossy(TEST_HTTP_PAYLOAD).as_ref(),
        );
    }
}

impl Drop for TestHttpServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn handle_test_http_connection(mut stream: TcpStream, payload: &[u8]) {
    let mut buffer = [0; 4096];
    let read = stream.read(&mut buffer).unwrap_or(0);
    let request = String::from_utf8_lossy(&buffer[..read]);
    let Some((start, end)) = request_range(&request) else {
        let _ = stream.write_all(
            b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );
        return;
    };

    if start >= payload.len() {
        let response = format!(
            "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            payload.len()
        );
        let _ = stream.write_all(response.as_bytes());
        return;
    }

    let end = end.min(payload.len() - 1);
    let body = &payload[start..=end];
    let response = format!(
        "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {start}-{end}/{}\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n",
        payload.len(),
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.write_all(body);
}

fn request_range(request: &str) -> Option<(usize, usize)> {
    let range = request.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case("range") {
            value.trim().strip_prefix("bytes=")
        } else {
            None
        }
    })?;
    let (start, end) = range.split_once('-')?;
    Some((start.parse().ok()?, end.parse().ok()?))
}

fn run_command(command: &mut Command) -> Result<()> {
    let debug = format!("{command:?}");
    let status = command
        .status()
        .map_err(|error| format!("{debug} failed to start: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{debug} exited with {status}").into())
    }
}

fn output_text(command: &mut Command) -> Result<String> {
    let debug = format!("{command:?}");
    let output = command
        .output()
        .map_err(|error| format!("{debug} failed to start: {error}"))?;
    if !output.status.success() {
        return Err(format!("{debug} exited with {}", output.status).into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn ensure_dir(path: impl AsRef<Path>) -> Result<()> {
    fs::create_dir_all(repo_root().join(path))?;
    Ok(())
}

fn remove_dir_if_exists(path: impl AsRef<Path>) -> Result<()> {
    let path = repo_root().join(path);
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

fn copy_file(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    let root = repo_root();
    let src = root.join(src);
    let dst = root.join(dst);
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(src, dst)?;
    Ok(())
}

fn copy_file_if_exists(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    let root = repo_root();
    let src = root.join(src);
    if src.is_file() {
        let dst = root.join(dst);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
    }
    Ok(())
}

fn generate_header() -> Result<()> {
    run_command(repo_command("cbindgen").args([
        "--config",
        "cbindgen.toml",
        "--crate",
        "takanawa-ffi",
        "--output",
        "include/takanawa.h",
    ]))
}

fn android_sdk() -> Result<()> {
    let mut license_input = Vec::new();
    for _ in 0..128 {
        license_input.extend_from_slice(b"y\n");
    }

    let mut licenses = repo_command("sdkmanager")
        .arg("--licenses")
        .stdin(Stdio::piped())
        .spawn()?;
    licenses
        .stdin
        .as_mut()
        .expect("sdkmanager stdin should be piped")
        .write_all(&license_input)?;
    let status = licenses.wait()?;
    if !status.success() {
        return Err(format!("sdkmanager --licenses exited with {status}").into());
    }

    run_command(repo_command("sdkmanager").args([
        "platforms;android-36",
        "build-tools;36.0.0",
        "ndk;28.2.13676358",
    ]))
}

fn cargo_check() -> Result<()> {
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

fn build_android() -> Result<()> {
    run_command(repo_command("rustup").args([
        "target",
        "add",
        "aarch64-linux-android",
        "armv7-linux-androideabi",
        "x86_64-linux-android",
        "i686-linux-android",
    ]))?;
    run_command(repo_command("cargo").args([
        "ndk",
        "-t",
        "arm64-v8a",
        "-t",
        "armeabi-v7a",
        "-t",
        "x86_64",
        "-t",
        "x86",
        "--platform",
        "23",
        "-o",
        "target/android/jniLibs",
        "build",
        "-p",
        "takanawa-ffi",
        "--release",
        "--locked",
        "--features",
        "jni",
    ]))
}

fn build_apple_xcframework() -> Result<()> {
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

fn verify_apple_xcframework(xcframework: &Path) -> Result<()> {
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

fn deployment_env(mut command: Command) -> Command {
    command.env(
        "IPHONEOS_DEPLOYMENT_TARGET",
        env::var("IPHONEOS_DEPLOYMENT_TARGET").unwrap_or_else(|_| "13.0".to_owned()),
    );
    command.env(
        "IPHONESIMULATOR_DEPLOYMENT_TARGET",
        env::var("IPHONESIMULATOR_DEPLOYMENT_TARGET").unwrap_or_else(|_| "13.0".to_owned()),
    );
    command.env(
        "MACOSX_DEPLOYMENT_TARGET",
        env::var("MACOSX_DEPLOYMENT_TARGET").unwrap_or_else(|_| "10.15".to_owned()),
    );
    command
}

fn build_windows_ffi() -> Result<()> {
    let target = env::var("TAKANAWA_WINDOWS_TARGET")
        .map_err(|_| "TAKANAWA_WINDOWS_TARGET is required for build-windows-ffi")?;
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

fn dist_windows() -> Result<()> {
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

fn dist_linux() -> Result<()> {
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

fn dist_macos_universal() -> Result<()> {
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

fn dist_apple_swiftpm() -> Result<()> {
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

fn dist_android() -> Result<()> {
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

fn check_csharp() -> Result<()> {
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

fn prepend_dynamic_library_path(command: &mut Command, native_dir: &Path) {
    let path = env::var_os("PATH").unwrap_or_default();
    let mut paths = env::split_paths(&path).collect::<Vec<_>>();
    paths.insert(0, native_dir.to_path_buf());
    if let Ok(joined) = env::join_paths(paths) {
        command.env("PATH", joined);
    }

    if cfg!(target_os = "macos") {
        prepend_env_path(command, "DYLD_LIBRARY_PATH", native_dir);
    } else if cfg!(target_os = "linux") {
        prepend_env_path(command, "LD_LIBRARY_PATH", native_dir);
    }
}

fn prepend_env_path(command: &mut Command, name: &str, native_dir: &Path) {
    let mut paths = env::var_os(name)
        .map(|value| env::split_paths(&value).collect::<Vec<_>>())
        .unwrap_or_default();
    paths.insert(0, native_dir.to_path_buf());
    if let Ok(joined) = env::join_paths(paths) {
        command.env(name, joined);
    }
}

fn pack_csharp() -> Result<()> {
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
        "runtimes/win-x86/native/takanawa_ffi.dll",
        "runtimes/win-arm64/native/takanawa_ffi.dll",
        "runtimes/linux-x64/native/libtakanawa_ffi.so",
        "runtimes/osx-x64/native/libtakanawa_ffi.dylib",
        "runtimes/osx-arm64/native/libtakanawa_ffi.dylib",
        "runtimes/android-arm64/native/libtakanawa_ffi.so",
        "runtimes/android-arm/native/libtakanawa_ffi.so",
        "runtimes/android-x86/native/libtakanawa_ffi.so",
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

fn prepare_csharp_nuget_assets() -> Result<()> {
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
        "takanawa-windows-i686.zip",
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

fn nuget_publish(mode: &str) -> Result<()> {
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

fn npm_publish(mode: &str) -> Result<()> {
    if mode != "dry-run" && mode != "publish" {
        return Err("usage: xtask npm-publish <dry-run|publish>".into());
    }

    let root = repo_root();
    let npm_cache = env::var_os("NPM_CONFIG_CACHE")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target/npm-cache"));
    fs::create_dir_all(&npm_cache)?;

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

    for package in &packages {
        println!("::group::pnpm --filter {} build", package.name);
        run_command(repo_command("pnpm").args(["--filter", package.name.as_str(), "build"]))?;
        println!("::endgroup::");
    }

    for package in &packages {
        let package_dir = format!("./{}", package.dir);
        if mode == "dry-run" {
            println!("::group::npm pack --dry-run {package_dir}");
            run_command(npm_command(&npm_cache).args([
                "pack",
                package_dir.as_str(),
                "--dry-run",
            ]))?;
            println!("::endgroup::");
            continue;
        }

        if npm_package_version_exists(&npm_cache, &package.name, &package.version)? {
            println!(
                "::notice title=Already published::{} {} already exists on npm; skipping.",
                package.name, package.version
            );
            continue;
        }

        let mut args = vec!["publish", package_dir.as_str(), "--provenance"];
        if package.name.starts_with('@') {
            args.push("--access");
            args.push("public");
        }
        println!("::group::npm publish {}@{}", package.name, package.version);
        run_command(npm_command(&npm_cache).args(args))?;
        println!("::endgroup::");
    }

    Ok(())
}

fn npm_command(npm_cache: &Path) -> Command {
    let mut command = repo_command("npm");
    command.env("NPM_CONFIG_CACHE", npm_cache);
    command
}

fn npm_package_version_exists(npm_cache: &Path, name: &str, version: &str) -> Result<bool> {
    let status = npm_command(npm_cache)
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

fn package_swiftpm() -> Result<()> {
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

fn test_capacitor_ios() -> Result<()> {
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

fn test_cmake_integration() -> Result<()> {
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

fn test_swift_integration() -> Result<()> {
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

fn check_apple() -> Result<()> {
    test_capacitor_ios()?;
    test_swift_integration()
}

fn swiftpm_release_manifest(version_arg: Option<String>) -> Result<()> {
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

fn publish_android_local() -> Result<()> {
    run_command(repo_command("./gradlew").args([
        "-Ptakanawa.skipRustBuild=true",
        ":takanawa-android:publishToMavenLocal",
        ":takanawa-android:verifyMavenLocalPublication",
    ]))?;
    run_command(repo_command("./gradlew").arg(":android-maven-local-smoke:assembleDebug"))
}

fn publish_android_central() -> Result<()> {
    run_command(repo_command("./gradlew").args([
        "-Ptakanawa.skipRustBuild=true",
        ":takanawa-android:publishAndReleaseToMavenCentral",
    ]))
}

fn validate_maven_central_env() -> Result<()> {
    let required = [
        "MAVEN_CENTRAL_USERNAME",
        "MAVEN_CENTRAL_PASSWORD",
        "SIGNING_IN_MEMORY_KEY",
        "SIGNING_IN_MEMORY_KEY_ID",
    ];
    let missing = required
        .iter()
        .copied()
        .filter(|name| env::var(name).map_or(true, |value| value.is_empty()))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("missing Maven Central secrets: {}", missing.join(", ")).into())
    }
}

fn github_release() -> Result<()> {
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

fn publish_crates() -> Result<()> {
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

fn sync_version(version: Option<String>) -> Result<()> {
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

fn workspace_version() -> Result<String> {
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

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    if !src.is_dir() {
        return Err(format!("missing directory {}", src.display()).into());
    }
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = dst.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path)?;
        if metadata.is_dir() {
            copy_dir(&source_path, &target_path)?;
        } else if metadata.is_file() {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&source_path, &target_path)?;
        } else if metadata.file_type().is_symlink() {
            copy_symlink(&source_path, &target_path)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn copy_symlink(src: &Path, dst: &Path) -> Result<()> {
    use std::os::unix::fs::symlink;

    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    if dst.exists() {
        fs::remove_file(dst)?;
    }
    symlink(fs::read_link(src)?, dst)?;
    Ok(())
}

#[cfg(windows)]
fn copy_symlink(src: &Path, dst: &Path) -> Result<()> {
    if src.is_dir() {
        copy_dir(src, dst)
    } else {
        fs::copy(src, dst)?;
        Ok(())
    }
}
