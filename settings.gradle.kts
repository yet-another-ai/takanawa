pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.PREFER_SETTINGS)
    repositories {
        mavenLocal()
        google()
        mavenCentral()
    }
}

rootProject.name = "takanawa-android-root"

include(":takanawa-android")
project(":takanawa-android").projectDir = file("android/takanawa-android")

val capacitorAndroidProjectDir = listOf(
    file("packages/takanawa-capacitor/node_modules/@capacitor/android/capacitor"),
    file("node_modules/@capacitor/android/capacitor"),
).firstOrNull { it.isDirectory }

val requestedTasks = gradle.startParameter.taskNames
val shouldIncludeCapacitorProjects = requestedTasks.isEmpty() || requestedTasks.any {
    it.contains("takanawa-capacitor") || it.contains("capacitor-android")
}

if (capacitorAndroidProjectDir != null && shouldIncludeCapacitorProjects) {
    include(":capacitor-android")
    project(":capacitor-android").projectDir = capacitorAndroidProjectDir

    include(":takanawa-capacitor")
    project(":takanawa-capacitor").projectDir = file("packages/takanawa-capacitor/android")
}

include(":android-maven-local-smoke")
project(":android-maven-local-smoke").projectDir = file("fixtures/android-maven-local-smoke")
