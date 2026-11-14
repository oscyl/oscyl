package io.github.oscyl.i18n

import java.text.NumberFormat

actual fun formatInteger(value: Int): String = NumberFormat.getIntegerInstance().format(value)
