plugins {
    id("com.android.application")
}

android {
    namespace = "ai.yetanother.takanawa.smoke"
    compileSdk = 36

    defaultConfig {
        applicationId = "ai.yetanother.takanawa.smoke"
        minSdk = 23
        targetSdk = 36
        versionCode = 1
        versionName = rootProject.version.toString()
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    implementation("ai.yetanother:takanawa-android:${rootProject.version}")
}
