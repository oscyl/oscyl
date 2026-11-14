package io.github.oscyl.preview

import android.text.TextUtils
import android.view.View
import com.github.takahirom.roborazzi.AndroidComposePreviewTester
import com.github.takahirom.roborazzi.ComposePreviewTester
import com.github.takahirom.roborazzi.ComposePreviewTester.TestParameter.JUnit4TestParameter.AndroidPreviewJUnit4TestParameter
import com.github.takahirom.roborazzi.ExperimentalRoborazziApi
import java.util.Locale
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.unit.LayoutDirection
import com.github.takahirom.roborazzi.RoborazziComposeComposableOption

/**
 * Captures every @Preview as a golden, honouring its `locale` the way a device does.
 * Robolectric's locale qualifier changes the configuration only: Compose resources read
 * the JVM default locale, and right-to-left locales need their layout direction. Both are set here.
 */
@OptIn(ExperimentalRoborazziApi::class)
class OscylPreviewTester : ComposePreviewTester<AndroidPreviewJUnit4TestParameter> by AndroidComposePreviewTester(
    capturer = object : AndroidComposePreviewTester.Capturer {
        override fun capture(parameter: AndroidComposePreviewTester.CaptureParameter) {
            val tag = parameter.preview.previewInfo.locale
            val previous = Locale.getDefault()
            var options = parameter.roborazziComposeOptions
            if (tag.isNotEmpty()) {
                val locale = Locale.forLanguageTag(tag.replace("-r", "-"))
                Locale.setDefault(locale)
                if (TextUtils.getLayoutDirectionFromLocale(locale) == View.LAYOUT_DIRECTION_RTL) {
                    options = options.builder().addOption(RightToLeft).build()
                }
            }
            try {
                AndroidComposePreviewTester.DefaultCapturer().capture(parameter.copy(roborazziComposeOptions = options))
            } finally {
                Locale.setDefault(previous)
            }
        }
    },
)

/** Lays the preview out right to left (Robolectric's `ldrtl` does not reach Compose). */
@OptIn(ExperimentalRoborazziApi::class)
private object RightToLeft : RoborazziComposeComposableOption {
    override fun configureWithComposable(content: @Composable () -> Unit): @Composable () -> Unit = {
        CompositionLocalProvider(LocalLayoutDirection provides LayoutDirection.Rtl, content)
    }
}
