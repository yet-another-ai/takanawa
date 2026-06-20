use std::env;
use std::io::Write;
use std::process::Stdio;

use crate::support::{Result, repo_command, run_command};

pub(crate) fn android_sdk() -> Result<()> {
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

pub(crate) fn build_android() -> Result<()> {
    run_command(repo_command("rustup").args([
        "target",
        "add",
        "aarch64-linux-android",
        "x86_64-linux-android",
    ]))?;
    run_command(repo_command("cargo").args([
        "ndk",
        "-t",
        "arm64-v8a",
        "-t",
        "x86_64",
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

pub(crate) fn publish_android_local() -> Result<()> {
    run_command(repo_command("./gradlew").args([
        "-Ptakanawa.skipRustBuild=true",
        ":takanawa-android:publishToMavenLocal",
        ":takanawa-android:verifyMavenLocalPublication",
    ]))?;
    run_command(repo_command("./gradlew").args([
        ":android-maven-local-smoke:assembleDebug",
        ":android-maven-local-smoke:assembleDebugAndroidTest",
    ]))
}

pub(crate) fn test_android_maven_local() -> Result<()> {
    publish_android_local()?;
    run_command(
        repo_command("./gradlew").arg(":android-maven-local-smoke:connectedDebugAndroidTest"),
    )
}

pub(crate) fn publish_android_central() -> Result<()> {
    run_command(repo_command("./gradlew").args([
        "-Ptakanawa.skipRustBuild=true",
        ":takanawa-android:publishAndReleaseToMavenCentral",
    ]))
}

pub(crate) fn validate_maven_central_env() -> Result<()> {
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
