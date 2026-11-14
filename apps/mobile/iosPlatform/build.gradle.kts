// The iOS platform code that needs no Rust core: CoreMIDI, the document picker and
// share sheet, haptics, storage and the language. It has no cinterop of its own (only
// the Kotlin/Native platform libraries), so its klibs build on Linux too
// (`kotlin.native.enableKlibsCrossCompilation`): `scripts/ci.sh` compiles it there,
// while `:shared` (whose UniFFI bindings need cinterop) builds for iOS on macOS only.
plugins {
    alias(libs.plugins.kotlinMultiplatform)
}

kotlin {
    jvmToolchain(21)
    iosArm64()
    iosSimulatorArm64()

    sourceSets {
        iosMain.dependencies {
            implementation(libs.kotlinx.coroutines.core)
        }
        iosTest.dependencies {
            implementation(libs.kotlin.test)
        }
    }
    compilerOptions {
        allWarningsAsErrors = true
        optIn.addAll("kotlinx.cinterop.ExperimentalForeignApi", "kotlinx.cinterop.BetaInteropApi")
    }
}
