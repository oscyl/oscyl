import com.github.takahirom.roborazzi.ExperimentalRoborazziApi
import org.gradle.work.DisableCachingByDefault

plugins {
    alias(libs.plugins.androidApplication)
    alias(libs.plugins.composeCompiler)
    alias(libs.plugins.roborazzi)
    alias(libs.plugins.baselineprofile)
}

android {
    namespace = "io.github.oscyl"
    compileSdk = libs.versions.android.compileSdk.get().toInt()
    ndkVersion = libs.versions.android.ndk.get()

    defaultConfig {
        applicationId = "io.github.oscyl"
        minSdk = libs.versions.android.minSdk.get().toInt()
        targetSdk = libs.versions.android.targetSdk.get().toInt()
        // From the workspace's Cargo.toml (see the root build).
        versionCode = rootProject.extra["oscylVersionCode"] as Int
        versionName = rootProject.extra["oscylVersion"] as String
        // Phones and the emulator; the Rust core is built for these two.
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    // A debug build beside an installed Oscyl (another signature, or data to keep):
    // ./gradlew :androidApp:assembleDebug -Poscyl.idSuffix=.redesign
    providers.gradleProperty("oscyl.idSuffix").orNull?.let { suffix ->
        buildTypes.getByName("debug").applicationIdSuffix = suffix
    }

    buildTypes {
        release {
            // R8 (compat mode, see gradle.properties): shrinks, optimises and keeps only what the
            // keep rules and the Baseline Profile need.
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }

    // JNA's prebuilt dispatcher ships stripped.
    packaging { jniLibs { keepDebugSymbols += "**/libjnidispatch.so" } }

    // Robolectric reads the merged resources and assets (Compose resources live there).
    testOptions {
        unitTests {
            isIncludeAndroidResources = true
            // Roborazzi's advice for faithful captures (and its warning otherwise).
            all { it.systemProperty("robolectric.pixelCopyRenderMode", "hardware") }
        }
        // The ATF gate: instrumented accessibility checks on a headless emulator Gradle
        // manages (an Automated Test Device image, x86_64):
        // ./gradlew :androidApp:pixel6Api34DebugAndroidTest
        // Where the emulator's software GPU crashes (emulator 36.6 on Linux), add
        // -Pandroid.testoptions.manageddevices.emulator.gpu=host.
        managedDevices {
            localDevices {
                create("pixel6Api34") {
                    device = "Pixel 6"
                    apiLevel = 34
                    systemImageSource = "aosp-atd"
                    testedAbi = "x86_64"
                }
            }
        }
    }
}

// Goldens are committed beside the tests.
roborazzi {
    outputDir = layout.projectDirectory.dir("src/test/snapshots")
    // Every @Preview in the app becomes a golden: previews are the single source of screenshots.
    @OptIn(ExperimentalRoborazziApi::class)
    generateComposePreviewRobolectricTests {
        enable = true
        packages = listOf("io.github.oscyl")
        includePrivatePreviews = true
        useScanOptionParametersInTester = true
        robolectricConfig = mapOf("sdk" to "[36]")
        testerQualifiedClassName = "io.github.oscyl.preview.OscylPreviewTester"
    }
}

kotlin {
    jvmToolchain(21)
}

// The Rust core built for this machine, so JVM unit tests (Robolectric) call
// the real bindings.
val hostFfi = configurations.dependencyScope("hostFfi")
val hostFfiClasspath = configurations.resolvable("hostFfiClasspath") {
    extendsFrom(hostFfi.get())
    attributes { attribute(Usage.USAGE_ATTRIBUTE, objects.named("oscyl-host-ffi")) }
}

dependencies {
    implementation(projects.shared)
    hostFfi(projects.shared)
    implementation(libs.activity.compose)
    implementation(libs.androidx.core)
    // Installs the Baseline Profile on devices without Play's cloud profiles.
    implementation(libs.profileinstaller)
    baselineProfile(project(":baselineprofile"))
    testImplementation(libs.junit)
    testImplementation(libs.jna)
    testImplementation(libs.androidx.test.junit)
    testImplementation(libs.robolectric)
    testImplementation(libs.roborazzi)
    testImplementation(libs.roborazzi.compose)
    testImplementation(libs.roborazzi.compose.preview.scanner.support)
    testImplementation(libs.composable.preview.scanner)
    testImplementation(libs.compose.ui.test.junit4)
    // The tests build small scrolling pages of their own around the shared parts.
    testImplementation(libs.compose.foundation)
    debugImplementation(libs.compose.ui.test.manifest)
    androidTestImplementation(libs.androidx.test.runner)
    androidTestImplementation(libs.androidx.test.junit)
    androidTestImplementation(libs.compose.foundation)
    androidTestImplementation(libs.compose.ui.test.junit4)
    androidTestImplementation(libs.compose.ui.test.junit4.accessibility)
}

// A release needs -PreleaseBuild=true: without it :shared (a single-variant
// library) keeps the en-XA pseudo-locale and the Rust core is a debug build.
val requireReleaseBuild = tasks.register<RequireReleaseBuild>("requireReleaseBuild") {
    @Suppress("UNCHECKED_CAST")
    releaseBuild = rootProject.extra["isReleaseBuild"] as Provider<Boolean>
}
tasks.named { it == "preReleaseBuild" }.configureEach { dependsOn(requireReleaseBuild) }
// And never package it in a release, even from stale outputs of a build
// without the flag (Compose's prepared resources keep removed qualifiers).
androidComponents {
    onVariants(selector().withBuildType("release")) { variant ->
        variant.androidResources.ignoreAssetsPatterns.add("<dir>values-en-rXA")
    }
}

// The benchmark variants the Baseline Profile plugin derives from release install as
// io.github.oscyl.bench: their own data, beside any other Oscyl on the phone.
androidComponents {
    onVariants { variant ->
        val type = variant.buildType.orEmpty()
        if (type.startsWith("benchmark") || type.startsWith("nonMinified")) variant.applicationId.set("io.github.oscyl.bench")
    }
}

// The launch window's ground, from design/tokens.json (light, and dark under values-night).
val generateWindowColors = tasks.register<GenerateWindowColors>("generateWindowColors") {
    tokens = layout.projectDirectory.file("../../../design/tokens.json")
}
androidComponents {
    onVariants { variant ->
        variant.sources.res?.addGeneratedSourceDirectory(generateWindowColors, GenerateWindowColors::outputDirectory)
    }
}

// -Poscyl.goldens=false leaves the preview screenshots out: the behaviour tests alone run
// in seconds (the goldens are nine tenths of the suite's time). For the loop while changing
// code that draws nothing new; scripts/ci.sh runs the goldens whenever UI sources changed.
val goldens = providers.gradleProperty("oscyl.goldens").map { it.toBoolean() }.orElse(true)

tasks.withType<Test>().configureEach {
    if (!goldens.get()) filter.excludeTestsMatching("com.github.takahirom.roborazzi.*")
    // Robolectric classes in parallel JVMs (half the cores), each with room for its sandbox.
    maxParallelForks = (Runtime.getRuntime().availableProcessors() / 2).coerceAtLeast(1)
    maxHeapSize = "2g"
    // Robolectric's Android 16 (SDK 36) sandbox reaches into the JDK's FileDescriptor internals.
    jvmArgs("--add-opens=java.base/jdk.internal.access=ALL-UNNAMED")
    // UniFFI's object cleaner uses Android's SystemCleaner, which Robolectric backs with the JDK's.
    jvmArgs("--add-exports=java.base/jdk.internal.ref=ALL-UNNAMED")
    jvmArgumentProviders.add(objects.newInstance<HostFfiLibrary>().apply { library.from(hostFfiClasspath) })
}

/** Points the bindings at the host libraries (UniFFI's per-component override). */
abstract class HostFfiLibrary : CommandLineArgumentProvider {
    @get:InputFiles
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val library: ConfigurableFileCollection

    override fun asArguments(): List<String> {
        // Cargo built only for this machine: its directory names the platform.
        val ffi = library.asFileTree.filter { it.name.contains("oscyl_ffi") }.files
        require(ffi.size == 1) { "expected one host oscyl_ffi library in ${library.files}, found $ffi" }
        val dir = ffi.single().parentFile
        val runtime = dir.listFiles().orEmpty().filter { it.name.contains("uniffi_runtime") }
        require(runtime.size == 1) { "expected one uniffi_runtime library in $dir, found $runtime" }
        return listOf(
            "-Duniffi.component.oscyl_ffi.libraryOverride=${ffi.single()}",
            "-Duniffi.component.uniffi_runtime.libraryOverride=${runtime.single()}",
        )
    }
}

/** Writes `oscyl_ground` for the light and dark launch windows from the colour tokens. */
@CacheableTask
abstract class GenerateWindowColors : DefaultTask() {
    @get:InputFile
    @get:PathSensitive(PathSensitivity.NONE)
    abstract val tokens: RegularFileProperty

    @get:OutputDirectory
    abstract val outputDirectory: DirectoryProperty

    @TaskAction
    fun generate() {
        @Suppress("UNCHECKED_CAST")
        val themes = (groovy.json.JsonSlurper().parse(tokens.get().asFile) as Map<String, Any>)
            .getValue("color") as Map<String, Map<String, String>>
        val out = outputDirectory.get().asFile
        out.deleteRecursively()
        for ((dir, theme) in listOf("values" to "light", "values-night" to "dark")) {
            val ground = themes.getValue(theme).getValue("ground")
            require(Regex("#[0-9a-fA-F]{6}").matches(ground)) { "color.$theme.ground must be #rrggbb, was $ground" }
            out.resolve(dir).apply { mkdirs() }.resolve("oscyl_window.xml").writeText(
                """
                |<?xml version="1.0" encoding="utf-8"?>
                |<!-- Generated from design/tokens.json by :androidApp:generateWindowColors. Do not edit. -->
                |<resources>
                |    <color name="oscyl_ground">$ground</color>
                |</resources>
                |""".trimMargin(),
            )
        }
    }
}

/** Fails unless the build was started with -PreleaseBuild=true. */
@DisableCachingByDefault(because = "A check with no output")
abstract class RequireReleaseBuild : DefaultTask() {
    @get:Input
    abstract val releaseBuild: Property<Boolean>

    @TaskAction
    fun check() {
        check(releaseBuild.get()) {
            "A release build needs -PreleaseBuild=true (release Rust core, no en-XA pseudo-locale)."
        }
    }
}
