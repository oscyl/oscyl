package io.github.oscyl.i18n

import platform.Foundation.NSNumber
import platform.Foundation.NSNumberFormatter
import platform.Foundation.NSNumberFormatterDecimalStyle

actual fun formatInteger(value: Int): String =
    NSNumberFormatter().apply { numberStyle = NSNumberFormatterDecimalStyle }
        .stringFromNumber(NSNumber(int = value)) ?: value.toString()
