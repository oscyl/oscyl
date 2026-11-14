// Plugins are declared here once, without applying them, so that every
// module loads them from the same class loader.
plugins {
    alias(libs.plugins.kotlinMultiplatform) apply false
    alias(libs.plugins.androidApplication) apply false
    alias(libs.plugins.androidKotlinMultiplatformLibrary) apply false
    alias(libs.plugins.composeMultiplatform) apply false
    alias(libs.plugins.composeCompiler) apply false
    alias(libs.plugins.uniffi) apply false
    alias(libs.plugins.roborazzi) apply false
    alias(libs.plugins.aboutlibraries) apply false
    alias(libs.plugins.androidTest) apply false
    alias(libs.plugins.baselineprofile) apply false
}

// The release version, from the one place it is written (Cargo.toml's
// [workspace.package]), through the one parser every release check uses:
// scripts/version.sh. It fails the build, with its reason, on a version that
// is not X.Y.Z or X.Y.Z-rc.N.
val versionInfo: Map<String, String> = providers.exec {
    commandLine(rootDir.resolve("../../scripts/version.sh").path)
    isIgnoreExitValue = true
}.let { run ->
    val exit = run.result.get().exitValue
    check(exit == 0) { "scripts/version.sh: ${run.standardError.asText.get().trim()}" }
    run.standardOutput.asText.get().lines().filter { '=' in it }.associate { it.substringBefore('=') to it.substringAfter('=') }
}
extra["oscylVersion"] = versionInfo.getValue("version")
extra["oscylVersionCode"] = versionInfo.getValue("code").toInt()

// -PreleaseBuild=true, read once for every module: a release Rust core, no en-XA
// pseudo-locale, and the only way :androidApp builds its release variant.
extra["isReleaseBuild"] = providers.gradleProperty("releaseBuild").map(String::toBoolean).orElse(false)
