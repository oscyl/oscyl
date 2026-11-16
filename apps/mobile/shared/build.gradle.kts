import ch.ubique.uniffi.plugin.tasks.MergeLibrariesTask
import javax.inject.Inject
import org.gradle.work.DisableCachingByDefault

plugins {
    alias(libs.plugins.kotlinMultiplatform)
    alias(libs.plugins.androidKotlinMultiplatformLibrary)
    alias(libs.plugins.composeMultiplatform)
    alias(libs.plugins.composeCompiler)
    alias(libs.plugins.uniffi)
    alias(libs.plugins.aboutlibraries)
}

kotlin {
    jvmToolchain(21)

    android {
        namespace = "io.github.oscyl.shared"
        compileSdk = libs.versions.android.compileSdk.get().toInt()
        minSdk = libs.versions.android.minSdk.get().toInt()
        androidResources { enable = true }
        withHostTest { }
    }
    listOf(iosArm64(), iosSimulatorArm64()).forEach { target ->
        // The Xcode project links this framework (embedAndSignAppleFrameworkForXcode).
        // The Rust core's static library is linked into it by the UniFFI plugin's cinterop.
        target.binaries.framework {
            baseName = "Shared"
            isStatic = true
        }
    }

    sourceSets {
        commonMain.dependencies {
            api(libs.compose.runtime)
            implementation(libs.compose.foundation)
            api(libs.compose.ui)
            implementation(libs.compose.material3)
            // Public: parts take drawable resources (the stage rail's icons).
            api(libs.compose.components.resources)
            // Back folds the review sheet.
            implementation(libs.compose.ui.backhandler)
            api(libs.navigation3.ui)
            // SynthModel is a ViewModel the app module creates.
            api(libs.lifecycle.viewmodel.compose)
            implementation(libs.material3.adaptive)
            implementation(libs.material3.adaptive.layout)
            implementation(libs.material3.adaptive.navigation)
            implementation(libs.compose.ui.tooling.preview)
            implementation(libs.kotlinx.serialization.json)
            implementation(libs.atomicfu)
        }
        iosMain.dependencies {
            // CoreMIDI, the document picker, haptics and storage (no Rust core: builds on Linux too).
            implementation(project(":iosPlatform"))
        }
        commonTest.dependencies {
            implementation(libs.kotlin.test)
        }
    }
}

// Lets Android Studio render the commonMain previews.
dependencies { androidRuntimeClasspath(libs.compose.ui.tooling) }

uniffi {
    generateFromLibrary {
        packageName = "io.github.oscyl.core"
    }
}

cargo {
    packageDirectory = layout.projectDirectory.dir("../../../crates/oscyl-ffi")
    ndkVersion = libs.versions.android.ndk
    androidDebugAbis.addAll("arm64-v8a", "x86_64")
}

// The Rust core built for this machine, beside the UniFFI runtime's own host
// library, for the app module's JVM unit tests. (The plugin's Android host
// test resources lack the runtime's library: its Android AAR has only ABIs.)
val uniffiRuntimeJvm = configurations.dependencyScope("uniffiRuntimeJvm")
val uniffiRuntimeJvmClasspath = configurations.resolvable("uniffiRuntimeJvmClasspath") {
    extendsFrom(uniffiRuntimeJvm.get())
    isTransitive = false
    attributes {
        attribute(Usage.USAGE_ATTRIBUTE, objects.named(Usage.JAVA_RUNTIME))
        attribute(Category.CATEGORY_ATTRIBUTE, objects.named(Category.LIBRARY))
        attribute(LibraryElements.LIBRARY_ELEMENTS_ATTRIBUTE, objects.named(LibraryElements.JAR))
    }
}
dependencies { uniffiRuntimeJvm(libs.uniffi.runtime.jvm) }

val assembleHostFfi = tasks.register<AssembleHostFfi>("assembleHostFfi") {
    crateLibraries.from(
        tasks.named<MergeLibrariesTask>("mergeUniffiAndroidHostTestResources").flatMap { it.outputDirectory },
    )
    runtimeJars.from(uniffiRuntimeJvmClasspath)
    outputDirectory = layout.buildDirectory.dir("generated/oscyl/hostFfi")
}
configurations.consumable("hostFfiElements") {
    attributes { attribute(Usage.USAGE_ATTRIBUTE, objects.named("oscyl-host-ffi")) }
    outgoing.artifact(assembleHostFfi.flatMap { it.outputDirectory })
}

// The Kotlin and Android libraries' notices for the About page, generated on
// every build into the Compose resources (files/aboutlibraries.json).
aboutLibraries {
    export {
        outputFile = layout.buildDirectory.file("generated/oscyl/aboutLibraries/aboutlibraries.json")
        variant = "android"
    }
}

// Oscyl's design tokens, generated into Kotlin from the one source every app reads.
val generateOscylTokens = tasks.register<GenerateOscylTokens>("generateOscylTokens") {
    tokens = layout.projectDirectory.file("../../../design/tokens.json")
    outputDirectory = layout.buildDirectory.dir("generated/oscyl/commonMain")
}
kotlin.sourceSets.commonMain { kotlin.srcDir(generateOscylTokens) }

// A pseudo-locale (en-XA) from the English strings, to see truncation, clipping
// and concatenation at a glance. Debug and test builds only.
val generatePseudoLocale = tasks.register<GeneratePseudoLocale>("generatePseudoLocale") {
    strings = layout.projectDirectory.file("src/commonMain/composeResources/values/strings.xml")
    outputDirectory = layout.buildDirectory.dir("generated/oscyl/pseudoLocale")
}

// The Compose resources: the committed ones, the program-title face (one
// committed copy, in the Linux app) and, unless -PreleaseBuild=true, the
// pseudo-locale. This library has a single Android variant, so the flag decides
// (it also builds the Rust core in release mode); :androidApp refuses to build
// its release variant without it.
@Suppress("UNCHECKED_CAST")
val isReleaseBuild = rootProject.extra["isReleaseBuild"] as Provider<Boolean>
val composeResourcesDir = layout.buildDirectory.dir("generated/oscyl/composeResources")
val assembleComposeResources = tasks.register<Sync>("assembleComposeResources") {
    from("src/commonMain/composeResources")
    from("../../linux/data/fonts/AlbertSans-Bold.ttf") {
        into("font")
        rename { "albert_sans_bold.ttf" }
    }
    from(tasks.named("exportLibraryDefinitions")) { into("files") }
    // Licence texts kept once in the repository: the font's, and Oscyl's own.
    from(files("../../linux/data/fonts/OFL-AlbertSans.txt", "../../../LICENSE-MIT", "../../../LICENSE-APACHE")) {
        into("files/licences")
    }
    // KORG's logue SDK licence: the oscillator engine unit holds its code.
    from("../../../units/oscyl-engine/sdk/LICENSE") {
        into("files/licences")
        rename { "BSD-3-Clause-logue-sdk.txt" }
    }
    if (!isReleaseBuild.get()) from(generatePseudoLocale)
    into(composeResourcesDir)
}
compose.resources {
    packageOfResClass = "io.github.oscyl.shared.generated.resources"
    customDirectory("commonMain", assembleComposeResources.map { composeResourcesDir.get() })
}

tasks.withType<Test>().configureEach {
    jvmArgumentProviders.add(
        objects.newInstance<StringsUnderTest>().apply {
            strings = layout.projectDirectory.file("src/commonMain/composeResources/values/strings.xml")
            pseudoLocale = generatePseudoLocale.flatMap { it.outputDirectory }
            composeResources.fileProvider(assembleComposeResources.map { it.destinationDir })
        },
    )
}

/** Tells the host tests where the English strings, the generated pseudo-locale and the assembled resources are. */
abstract class StringsUnderTest : CommandLineArgumentProvider {
    /** The assembled Compose resources (licence files among them). */
    @get:InputDirectory
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val composeResources: DirectoryProperty

    @get:InputFile
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val strings: RegularFileProperty

    @get:InputDirectory
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val pseudoLocale: DirectoryProperty

    override fun asArguments() = listOf(
        "-Doscyl.strings=${strings.get().asFile.path}",
        "-Doscyl.pseudoLocale=${pseudoLocale.get().asFile.path}",
        "-Doscyl.composeResources=${composeResources.get().asFile.path}",
    )
}

/** Writes `OscylTokens.kt`: colour roles per theme, shape and motion, from `tokens.json`. */
@CacheableTask
abstract class GenerateOscylTokens : DefaultTask() {
    @get:InputFile
    @get:PathSensitive(PathSensitivity.NONE)
    abstract val tokens: RegularFileProperty

    @get:OutputDirectory
    abstract val outputDirectory: DirectoryProperty

    @TaskAction
    fun generate() {
        @Suppress("UNCHECKED_CAST")
        val json = groovy.json.JsonSlurper().parse(tokens.get().asFile) as Map<String, Any>
        @Suppress("UNCHECKED_CAST")
        val shared = json.getValue("color") as Map<String, Map<String, String>>
        @Suppress("UNCHECKED_CAST")
        val shape = json.getValue("shape") as Map<String, Number>
        @Suppress("UNCHECKED_CAST")
        val motion = json.getValue("motion") as Map<String, Any>
        @Suppress("UNCHECKED_CAST")
        val mobile = json.getValue("mobile") as Map<String, Any>
        @Suppress("UNCHECKED_CAST")
        val floating = mobile.getValue("floating") as Map<String, Any>
        @Suppress("UNCHECKED_CAST")
        val knob = mobile.getValue("knob") as Map<String, Any>
        // The phone's own overrides of shared roles (`mobile.color`): the Linux app keeps the shared values.
        @Suppress("UNCHECKED_CAST")
        val overrides = (mobile["color"] as Map<String, Any>?).orEmpty().filterKeys { !it.startsWith("$") } as Map<String, Map<String, String>>
        val themes = shared.mapValues { (name, colors) ->
            val own = overrides[name].orEmpty()
            require(own.keys.all { it in colors }) { "mobile.color.$name overrides a role the shared theme lacks" }
            colors + own
        }
        require(overrides.keys.all { it in shared }) { "mobile.color names a theme the shared colours lack" }

        val roles = themes.values.first().keys.toList()
        themes.forEach { (name, colors) ->
            require(colors.keys.toList() == roles) { "theme '$name' must list the same roles as the others" }
        }
        val easing = Regex("""cubic-bezier\(([^)]*)\)""").matchEntire(motion.getValue("easing") as String)
            ?.groupValues?.get(1)?.split(",")?.map { it.trim().toFloat() }
            ?: error("motion.easing must be a cubic-bezier()")

        val out = StringBuilder()
        out.appendLine("// Generated from design/tokens.json by :shared:generateOscylTokens. Do not edit.")
        out.appendLine("package io.github.oscyl.theme")
        out.appendLine()
        out.appendLine("import androidx.compose.animation.core.CubicBezierEasing")
        out.appendLine("import androidx.compose.animation.core.Easing")
        out.appendLine("import androidx.compose.runtime.Immutable")
        out.appendLine("import androidx.compose.ui.graphics.Color")
        out.appendLine("import androidx.compose.ui.unit.Dp")
        out.appendLine("import androidx.compose.ui.unit.dp")
        out.appendLine()
        out.appendLine("/** One theme's colour roles. */")
        out.appendLine("@Immutable")
        out.appendLine("data class OscylColors(")
        roles.forEach { out.appendLine("    val ${camel(it)}: Color,") }
        out.appendLine(") {")
        out.appendLine("    /** Every role by its token name. */")
        out.appendLine("    internal val roles: Map<String, Color>")
        out.appendLine("        get() = mapOf(")
        roles.forEach { out.appendLine("            \"$it\" to ${camel(it)},") }
        out.appendLine("        )")
        out.appendLine("}")
        out.appendLine()
        out.appendLine("object OscylTokens {")
        themes.forEach { (name, colors) ->
            out.appendLine("    val ${name.replaceFirstChar(Char::uppercase)}: OscylColors = OscylColors(")
            colors.forEach { (role, value) -> out.appendLine("        ${camel(role)} = ${color(value)},") }
            out.appendLine("    )")
            out.appendLine()
        }
        out.appendLine("    val radius: Dp = ${shape.getValue("radius_px")}.dp")
        out.appendLine("    val radiusInner: Dp = ${shape.getValue("radius_inner_px")}.dp")
        out.appendLine("    val radiusPanel: Dp = ${shape.getValue("radius_panel_px")}.dp")
        out.appendLine("    val ruleWidth: Dp = ${shape.getValue("rule_px")}.dp")
        out.appendLine("    const val fastMs: Int = ${motion.getValue("fast_ms")}")
        out.appendLine("    const val baseMs: Int = ${motion.getValue("base_ms")}")
        out.appendLine("    const val slowMs: Int = ${motion.getValue("slow_ms")}")
        out.appendLine("    val easing: Easing = CubicBezierEasing(${easing.joinToString { "${it}f" }})")
        out.appendLine()
        out.appendLine("    /** The floating shadow: a close layer, a soft layer and the top inner highlight. */")
        out.appendLine("    object Floating {")
        floating.forEach { (key, value) ->
            val name = camel(key.replace('_', '-').removeSuffix("-dp"))
            when {
                key.endsWith("_dp") -> out.appendLine("        val $name: Dp = $value.dp")
                value is String -> out.appendLine("        val $name: Color = ${color(value)}")
                else -> out.appendLine("        const val $name: Float = ${value}f")
            }
        }
        out.appendLine("    }")
        out.appendLine()
        out.appendLine("    /** A knob's touch, travel, fine control and weight. */")
        out.appendLine("    object Knob {")
        knob.forEach { (key, value) ->
            val name = camel(key.replace('_', '-').removeSuffix("-dp"))
            if (key.endsWith("_dp")) out.appendLine("        val $name: Dp = $value.dp")
            else out.appendLine("        const val $name: Float = ${value}f")
        }
        out.appendLine("    }")
        out.appendLine()
        out.appendLine("    /** Space between the floating island and the stage rail's foot. */")
        out.appendLine("    val railAnchor: Dp = ${mobile.getValue("rail_anchor_dp")}.dp")
        out.appendLine("}")

        val dir = outputDirectory.get().asFile.resolve("io/github/oscyl/theme")
        dir.deleteRecursively()
        dir.mkdirs()
        dir.resolve("OscylTokens.kt").writeText(out.toString())
    }

    private fun camel(token: String) =
        token.split('-').mapIndexed { i, part -> if (i == 0) part else part.replaceFirstChar(Char::uppercase) }
            .joinToString("")

    /** `#rrggbb` or `rgb(r g b / a%)` as a Compose `Color(0xAARRGGBB)` literal. */
    private fun color(value: String): String {
        val argb = Regex("""#([0-9a-fA-F]{6})""").matchEntire(value)?.let { "FF" + it.groupValues[1] }
            ?: Regex("""rgb\((\d+) (\d+) (\d+) / (\d+)%\)""").matchEntire(value)?.groupValues?.let { g ->
                val alpha = Math.round(g[4].toInt() * 255 / 100.0).toInt()
                listOf(alpha, g[1].toInt(), g[2].toInt(), g[3].toInt()).joinToString("") { "%02X".format(it) }
            }
            ?: error("unsupported colour '$value'")
        return "Color(0x${argb.uppercase()})"
    }
}

/**
 * Writes `values-en-rXA/strings.xml`: each English string accented, about 40%
 * longer and bracketed, with format arguments and escapes left intact.
 */
@CacheableTask
abstract class GeneratePseudoLocale : DefaultTask() {
    @get:InputFile
    @get:PathSensitive(PathSensitivity.NONE)
    abstract val strings: RegularFileProperty

    @get:OutputDirectory
    abstract val outputDirectory: DirectoryProperty

    @TaskAction
    fun generate() {
        val factory = javax.xml.parsers.DocumentBuilderFactory.newInstance()
        val document = factory.newDocumentBuilder().parse(strings.get().asFile)
        val elements = document.documentElement.childNodes
        for (i in 0 until elements.length) {
            val element = elements.item(i) as? org.w3c.dom.Element ?: continue
            when (element.tagName) {
                "string" -> pseudo(element)
                "plurals", "string-array" -> {
                    val items = element.getElementsByTagName("item")
                    for (j in 0 until items.length) pseudo(items.item(j) as org.w3c.dom.Element)
                }
            }
        }
        val dir = outputDirectory.get().asFile
        dir.deleteRecursively()
        val out = dir.resolve("values-en-rXA/strings.xml")
        out.parentFile.mkdirs()
        val transformer = javax.xml.transform.TransformerFactory.newInstance().newTransformer()
        transformer.setOutputProperty(javax.xml.transform.OutputKeys.ENCODING, "UTF-8")
        transformer.transform(javax.xml.transform.dom.DOMSource(document), javax.xml.transform.stream.StreamResult(out))
    }

    private fun pseudo(element: org.w3c.dom.Element) {
        // Inline markup would be flattened into text: the app's strings have none, and must not.
        val markup = (0 until element.childNodes.length).map { element.childNodes.item(it) }.firstOrNull { it is org.w3c.dom.Element }
        check(markup == null) { "string '${element.getAttribute("name")}' holds markup <${markup?.nodeName}>: use plain text" }
        val text = element.textContent
        // Forty percent of the whole string (format arguments too: they widen once filled).
        val padding = filler.repeat(1 + text.length / filler.length).take((text.length * 2 + 4) / 5)
        val accented = untouchableOrChar.replace(text) { match ->
            match.groups[1]?.value ?: (accents[match.value[0]] ?: match.value[0]).toString()
        }
        element.textContent = "[$accented${if (padding.isEmpty()) "" else " $padding"}]"
    }

    private companion object {
        /** Format arguments (`%1$s`, `%d`) and backslash escapes keep their characters. */
        val untouchable = Regex("""%(\d+\$)?[-#+ 0,(]*\d*(\.\d+)?[a-zA-Z%]|\\.""")
        val untouchableOrChar = Regex("(${untouchable.pattern})|.", RegexOption.DOT_MATCHES_ALL)
        const val filler = "one two three four five six seven eight nine ten "
        val accents: Map<Char, Char> = (
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ" zip
                "àƀçðéƒĝĥîĵķļɱñöþǫŕšţûṽŵẋýžÀßÇÐÉƑĜĤÎĴĶĻṀÑÖÞǪŔŠŢÛṼŴẊÝŽ"
            ).toMap()
    }
}

/** The host libraries in JNA's layout (`linux-x86-64/lib….so`): the crate's and the runtime's. */
@DisableCachingByDefault(because = "Copying is cheaper than the cache")
abstract class AssembleHostFfi : DefaultTask() {
    @get:InputFiles
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val crateLibraries: ConfigurableFileCollection

    @get:InputFiles
    @get:PathSensitive(PathSensitivity.NONE)
    abstract val runtimeJars: ConfigurableFileCollection

    @get:OutputDirectory
    abstract val outputDirectory: DirectoryProperty

    @get:Inject
    abstract val files: FileSystemOperations

    @get:Inject
    abstract val archives: ArchiveOperations

    @TaskAction
    fun assemble() {
        files.sync {
            from(crateLibraries)
            runtimeJars.forEach { jar ->
                from(archives.zipTree(jar)) { include("*/libuniffi_runtime.*", "*/uniffi_runtime.dll") }
            }
            includeEmptyDirs = false
            into(outputDirectory)
        }
    }
}
