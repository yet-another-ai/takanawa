use std::env;

mod android;
mod apple;
mod csharp;
mod gdextension;
mod js;
mod native;
mod release;
mod support;

use support::Result;

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
        "android-sdk" => android::android_sdk(),
        "build-android" => android::build_android(),
        "build-apple-xcframework" => apple::build_apple_xcframework(),
        "build-gdextension-android" => gdextension::build_gdextension_android(),
        "build-gdextension-apple" => gdextension::build_gdextension_apple(),
        "build-gdextension-desktop" => gdextension::build_gdextension_desktop(),
        "build-gdextension-windows" => gdextension::build_gdextension_windows(),
        "build-windows-ffi" => native::build_windows_ffi(),
        "cargo-check" => cargo_check(),
        "check-apple" => apple::check_apple(),
        "check-csharp" => csharp::check_csharp(),
        "dist-android" => native::dist_android(),
        "dist-apple-swiftpm" => apple::dist_apple_swiftpm(),
        "dist-gdextension" => gdextension::dist_gdextension(),
        "dist-linux" => native::dist_linux(),
        "dist-macos-universal" => native::dist_macos_universal(),
        "dist-windows" => native::dist_windows(),
        "github-release" => release::github_release(),
        "npm-publish" => js::npm_publish(args.next().as_deref().unwrap_or("")),
        "nuget-publish" => csharp::nuget_publish(args.next().as_deref().unwrap_or("")),
        "package-swiftpm" => apple::package_swiftpm(),
        "pack-csharp" => csharp::pack_csharp(),
        "prepare-csharp-nuget-assets" => csharp::prepare_csharp_nuget_assets(),
        "publish-android-local" => android::publish_android_local(),
        "publish-android-central" => android::publish_android_central(),
        "publish-crates" => release::publish_crates(),
        "swiftpm-release-manifest" => apple::swiftpm_release_manifest(
            args.next().or_else(|| env::var("GITHUB_REF_NAME").ok()),
        ),
        "sync-version" => {
            let version = args.next();
            if args.next().is_some() {
                return Err("usage: xtask sync-version [version]".into());
            }
            release::sync_version(version)
        }
        "test-capacitor-ios" => apple::test_capacitor_ios(),
        "test-android-maven-local" => android::test_android_maven_local(),
        "test-cmake-integration" => native::test_cmake_integration(),
        "test-swift-integration" => apple::test_swift_integration(),
        "validate-maven-central-env" => android::validate_maven_central_env(),
        _ => {
            print_usage();
            Err(format!("unknown xtask command: {command}").into())
        }
    }
}

fn print_usage() {
    eprintln!(
        "usage: cargo run -p xtask -- <command>

         commands:
           android-sdk
           build-android
           build-apple-xcframework
           build-gdextension-android
           build-gdextension-apple
           build-gdextension-desktop
           build-gdextension-windows
           build-windows-ffi
           cargo-check
           check-apple
           check-csharp
           dist-android
           dist-apple-swiftpm
           dist-gdextension
           dist-linux
           dist-macos-universal
           dist-windows
           github-release
           npm-publish <dry-run|publish>
           nuget-publish <dry-run|publish>
           package-swiftpm
           pack-csharp
           prepare-csharp-nuget-assets
           publish-android-local
           publish-android-central
           publish-crates
           swiftpm-release-manifest [version]
           sync-version [version]
           test-capacitor-ios
           test-android-maven-local
           test-cmake-integration
           test-swift-integration
           validate-maven-central-env"
    );
}

fn cargo_check() -> Result<()> {
    native::cargo_check()
}
