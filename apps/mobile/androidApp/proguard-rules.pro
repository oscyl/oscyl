# R8 keep rules for the release build.
# JNA reaches Java classes from native code and by reflection (structures, callbacks).
-keep class com.sun.jna.** { *; }
-keep class * implements com.sun.jna.** { *; }
-dontwarn java.awt.**
# The UniFFI bindings to the Rust core: called through JNA by name.
-keep class io.github.oscyl.core.** { *; }
-keep class uniffi.** { *; }
