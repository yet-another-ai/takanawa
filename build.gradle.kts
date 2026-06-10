fun readWorkspaceVersion(): String {
    var inWorkspacePackage = false
    for (rawLine in rootProject.file("Cargo.toml").readLines()) {
        val line = rawLine.trim()
        if (line.startsWith("[") && line.endsWith("]")) {
            inWorkspacePackage = line == "[workspace.package]"
        } else if (inWorkspacePackage && line.startsWith("version")) {
            return line.substringAfter('"').substringBefore('"')
        }
    }
    error("Cargo.toml is missing [workspace.package] version")
}

val takanawaVersion = readWorkspaceVersion()

plugins {
    id("com.android.application") version "9.2.0" apply false
    id("com.android.library") version "9.2.0" apply false
    id("com.vanniktech.maven.publish") version "0.36.0" apply false
}

allprojects {
    group = "ai.yetanother"
    version = takanawaVersion
}
