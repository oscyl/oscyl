package io.github.oscyl.ui.shell

import androidx.navigation3.runtime.NavKey
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.destination_editor
import io.github.oscyl.shared.generated.resources.destination_generate
import io.github.oscyl.shared.generated.resources.destination_library
import io.github.oscyl.shared.generated.resources.ic_editor
import io.github.oscyl.shared.generated.resources.ic_generate
import io.github.oscyl.shared.generated.resources.ic_library
import org.jetbrains.compose.resources.DrawableResource
import org.jetbrains.compose.resources.StringResource

/** The island's three places, in island order; Back from the others returns to the Library. */
enum class Destination(val label: StringResource, val icon: DrawableResource) : NavKey {
    Library(Res.string.destination_library, Res.drawable.ic_library),
    Editor(Res.string.destination_editor, Res.drawable.ic_editor),
    Generate(Res.string.destination_generate, Res.drawable.ic_generate),
}
