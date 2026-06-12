import java.util.zip.ZipFile

plugins {
    id("com.android.library")
    id("com.vanniktech.maven.publish")
}

android {
    namespace = "ai.yetanother.takanawa"
    compileSdk = 36
    ndkVersion = "28.2.13676358"

    defaultConfig {
        minSdk = 23
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        consumerProguardFiles("consumer-rules.pro")
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    sourceSets {
        getByName("main") {
            jniLibs.directories.add("build/generated/takanawaJniLibs")
        }
    }

    testOptions {
        unitTests.isIncludeAndroidResources = false
    }
}

dependencies {
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.jetbrains.kotlin:kotlin-test:2.2.10")
    androidTestImplementation("androidx.test:core:1.6.1")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test:runner:1.6.2")
}

val buildAndroidRust by tasks.registering(Exec::class) {
    description = "Build Takanawa Android JNI libraries with cargo-ndk."
    group = "build"
    workingDir = rootProject.layout.projectDirectory.asFile
    commandLine("bash", "scripts/build-android.sh")
    onlyIf {
        !providers.gradleProperty("takanawa.skipRustBuild")
            .map(String::toBoolean)
            .getOrElse(false)
    }
}

val syncAndroidJniLibs by tasks.registering(Sync::class) {
    description = "Sync Rust-built JNI libraries into the Android AAR inputs."
    group = "build"
    dependsOn(buildAndroidRust)
    from(rootProject.layout.projectDirectory.dir("target/android/jniLibs"))
    into(layout.buildDirectory.dir("generated/takanawaJniLibs"))
    include("**/*.so")
}

tasks.named("preBuild") {
    dependsOn(syncAndroidJniLibs)
}

tasks.configureEach {
    if (name.startsWith("merge") && name.endsWith("JniLibFolders")) {
        dependsOn(syncAndroidJniLibs)
    }
}

tasks.register("verifyReleaseAar") {
    description = "Verify the release AAR contains all expected Android JNI libraries."
    group = "verification"
    dependsOn("bundleReleaseAar")

    val aar = layout.buildDirectory.file("outputs/aar/takanawa-android-release.aar")
    inputs.file(aar)

    doLast {
        val requiredEntries = listOf(
            "AndroidManifest.xml",
            "classes.jar",
            "jni/arm64-v8a/libtakanawa_ffi.so",
            "jni/armeabi-v7a/libtakanawa_ffi.so",
            "jni/x86/libtakanawa_ffi.so",
            "jni/x86_64/libtakanawa_ffi.so",
        )

        ZipFile(aar.get().asFile).use { zip ->
            val missing = requiredEntries.filter { zip.getEntry(it) == null }
            check(missing.isEmpty()) {
                "Missing expected AAR entries: ${missing.joinToString()}"
            }
        }
    }
}

tasks.register("verifyMavenLocalPublication") {
    description = "Verify Maven local publication artifacts required by Maven Central."
    group = "verification"
    dependsOn("publishToMavenLocal")

    doLast {
        val artifactVersion = project.version.toString()
        val artifactDir = file("${System.getProperty("user.home")}/.m2/repository/ai/yetanother/takanawa-android/$artifactVersion")
        val requiredFiles = listOf(
            "takanawa-android-$artifactVersion.aar",
            "takanawa-android-$artifactVersion.pom",
            "takanawa-android-$artifactVersion-sources.jar",
            "takanawa-android-$artifactVersion-javadoc.jar",
        )
        val missing = requiredFiles.filterNot { artifactDir.resolve(it).isFile }
        check(missing.isEmpty()) {
            "Missing expected Maven local artifacts in $artifactDir: ${missing.joinToString()}"
        }
    }
}

mavenPublishing {
    coordinates(project.group.toString(), "takanawa-android", project.version.toString())
    publishToMavenCentral()

    if (providers.gradleProperty("signingInMemoryKey").isPresent) {
        signAllPublications()
    }

    pom {
        name = "Takanawa Android"
        description = "Kotlin-first Android SDK for the Takanawa Rust range-download library."
        inceptionYear = "2026"
        url = "https://github.com/yet-another-ai/takanawa"
        licenses {
            license {
                name = "MIT License"
                url = "https://opensource.org/license/mit"
                distribution = "repo"
            }
        }
        developers {
            developer {
                id = "yetanother-ai"
                name = "yetanother.ai"
                email = "opensource@yetanother.ai"
            }
        }
        scm {
            url = "https://github.com/yet-another-ai/takanawa"
            connection = "scm:git:https://github.com/yet-another-ai/takanawa.git"
            developerConnection = "scm:git:ssh://git@github.com/yet-another-ai/takanawa.git"
        }
    }
}
