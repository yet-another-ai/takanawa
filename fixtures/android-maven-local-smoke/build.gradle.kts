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
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    implementation("ai.yetanother:takanawa-android:${rootProject.version}")

    androidTestImplementation("androidx.test:core:1.6.1")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test:runner:1.6.2")
}
