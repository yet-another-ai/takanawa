pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        mavenLocal()
        google()
        mavenCentral()
    }
}

rootProject.name = "takanawa-android-root"

include(":takanawa-android")
project(":takanawa-android").projectDir = file("android/takanawa-android")

include(":android-maven-local-smoke")
project(":android-maven-local-smoke").projectDir = file("fixtures/android-maven-local-smoke")
