// Baseline Profiles and frame benchmarks for :androidApp, run on a connected phone:
//   ./gradlew -PreleaseBuild=true :androidApp:generateBaselineProfile
//   ./gradlew -PreleaseBuild=true :baselineprofile:connectedBenchmarkReleaseAndroidTest
// The app under test is the benchmark variant, installed beside any other Oscyl as
// io.github.oscyl.bench (its own data; nobody's library is touched).
plugins {
    alias(libs.plugins.androidTest)
    alias(libs.plugins.baselineprofile)
}

android {
    namespace = "io.github.oscyl.baselineprofile"
    compileSdk = libs.versions.android.compileSdk.get().toInt()
    defaultConfig {
        minSdk = 28
        targetSdk = libs.versions.android.targetSdk.get().toInt()
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }
    targetProjectPath = ":androidApp"
}

baselineProfile {
    useConnectedDevices = true
}

dependencies {
    implementation(libs.androidx.test.junit)
    implementation(libs.androidx.test.runner)
    implementation(libs.uiautomator)
    implementation(libs.benchmark.macro.junit4)
}
