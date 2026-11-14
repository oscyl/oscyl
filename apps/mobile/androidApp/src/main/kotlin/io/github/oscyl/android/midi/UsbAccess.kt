package io.github.oscyl.android.midi

import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.hardware.usb.UsbDevice
import android.hardware.usb.UsbManager
import android.os.Build
import androidx.core.content.ContextCompat
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.coroutines.resume
import kotlinx.coroutines.suspendCancellableCoroutine

/** Android's USB access dialog for one device, as a suspending yes or no. */
internal object UsbAccess {
    suspend fun request(context: Context, device: UsbDevice): Boolean {
        val usb = context.getSystemService(UsbManager::class.java) ?: return false
        if (usb.hasPermission(device)) return true
        // One action per device, so two dialogs never answer each other.
        val action = "${context.packageName}.USB_ACCESS.${device.deviceId}"
        return suspendCancellableCoroutine { answer ->
            val finished = AtomicBoolean()
            lateinit var receiver: BroadcastReceiver
            fun finish() {
                if (finished.compareAndSet(false, true)) context.unregisterReceiver(receiver)
            }
            receiver = object : BroadcastReceiver() {
                override fun onReceive(context: Context, intent: Intent) {
                    finish()
                    answer.resume(intent.getBooleanExtra(UsbManager.EXTRA_PERMISSION_GRANTED, false))
                }
            }
            ContextCompat.registerReceiver(context, receiver, IntentFilter(action), ContextCompat.RECEIVER_NOT_EXPORTED)
            answer.invokeOnCancellation { finish() }
            // Mutable: the system adds the answer's extras. Explicit: only this app receives it.
            val flags = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) PendingIntent.FLAG_MUTABLE else 0
            val intent = PendingIntent.getBroadcast(context, device.deviceId, Intent(action).setPackage(context.packageName), flags)
            usb.requestPermission(device, intent)
        }
    }
}
