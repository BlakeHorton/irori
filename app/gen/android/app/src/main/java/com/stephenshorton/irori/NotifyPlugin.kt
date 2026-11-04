package com.stephenshorton.irori

import android.Manifest
import android.app.Activity
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.webkit.WebView
import androidx.core.app.ActivityCompat
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
class ShowArgs {
  lateinit var id: String
  var title: String = ""
  var body: String = ""
}

@InvokeArg
class CloseArgs {
  lateinit var id: String
}

// Card #395: the phone's notifications. The Rust notifier posts a Talk
// mention or DM here (show), and takes it down once it's read elsewhere
// (close). A tap opens Irori with the note's id in the intent; Rust takes
// it (takeTap) when the app resumes or the page starts listening, and opens
// that conversation.
@TauriPlugin
class NotifyPlugin(private val activity: Activity) : Plugin(activity) {
  private var tapped: String? = null

  override fun load(webView: WebView) {
    super.load(webView)
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      val ch = NotificationChannel(CHANNEL, "Mentions and direct messages", NotificationManager.IMPORTANCE_HIGH)
      ch.description = "A message that @-mentions you, or a direct message to you"
      activity.getSystemService(NotificationManager::class.java).createNotificationChannel(ch)
    }
    // Android 13+ asks once; until it's allowed, notes are dropped.
    if (Build.VERSION.SDK_INT >= 33 &&
      ContextCompat.checkSelfPermission(activity, Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
    ) {
      ActivityCompat.requestPermissions(activity, arrayOf(Manifest.permission.POST_NOTIFICATIONS), 395)
    }
    keep(activity.intent)
  }

  override fun onNewIntent(intent: Intent) {
    super.onNewIntent(intent)
    keep(intent)
  }

  private fun keep(intent: Intent?) {
    val id = intent?.getStringExtra(EXTRA_ID) ?: return
    tapped = id
    intent.removeExtra(EXTRA_ID)
  }

  @Command
  fun show(invoke: Invoke) {
    val a = invoke.parseArgs(ShowArgs::class.java)
    val open = Intent(activity, MainActivity::class.java)
      .addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP)
      .putExtra(EXTRA_ID, a.id)
    val pending = PendingIntent.getActivity(
      activity, a.id.hashCode(), open,
      PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
    )
    val n = NotificationCompat.Builder(activity, CHANNEL)
      .setSmallIcon(R.drawable.ic_notify)
      .setContentTitle(a.title)
      .setContentText(a.body)
      .setStyle(NotificationCompat.BigTextStyle().bigText(a.body))
      .setPriority(NotificationCompat.PRIORITY_HIGH)
      .setCategory(NotificationCompat.CATEGORY_MESSAGE)
      .setContentIntent(pending)
      .setAutoCancel(true)
      .build()
    val out = JSObject()
    try {
      NotificationManagerCompat.from(activity).notify(a.id, 0, n)
      out.put("shown", true)
    } catch (e: SecurityException) {
      // Notifications aren't allowed (Android 13+ without the permission).
      out.put("shown", false)
    }
    invoke.resolve(out)
  }

  @Command
  fun close(invoke: Invoke) {
    val a = invoke.parseArgs(CloseArgs::class.java)
    NotificationManagerCompat.from(activity).cancel(a.id, 0)
    invoke.resolve()
  }

  @Command
  fun takeTap(invoke: Invoke) {
    val out = JSObject()
    out.put("id", tapped ?: "")
    tapped = null
    invoke.resolve(out)
  }

  companion object {
    const val CHANNEL = "talk"
    const val EXTRA_ID = "irori.notification"
  }
}
