package com.stephenshorton.irori

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.provider.Settings
import androidx.core.content.FileProvider
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.File

@InvokeArg
class InstallArgs {
  lateinit var path: String
}

// Card #270: opens Android's package installer on an update APK the Rust
// updater downloaded and checked against SHA256SUMS. The APK sits under the
// app's files dir (irori/updates/), shared through the FileProvider declared
// in the manifest. When installing unknown apps isn't allowed yet, it opens
// that settings page for Irori instead and says so.
@TauriPlugin
class ApkInstallPlugin(private val activity: Activity) : Plugin(activity) {
  @Command
  fun install(invoke: Invoke) {
    val args = invoke.parseArgs(InstallArgs::class.java)
    val apk = File(args.path)
    if (!apk.isFile) {
      invoke.reject("no APK at ${args.path}")
      return
    }
    val out = JSObject()
    try {
      if (!activity.packageManager.canRequestPackageInstalls()) {
        activity.startActivity(
          Intent(
            Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
            Uri.parse("package:${activity.packageName}")
          )
        )
        out.put("outcome", "needs-permission")
        invoke.resolve(out)
        return
      }
      val uri = FileProvider.getUriForFile(activity, "${activity.packageName}.fileprovider", apk)
      activity.startActivity(
        Intent(Intent.ACTION_VIEW)
          .setDataAndType(uri, "application/vnd.android.package-archive")
          .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
      )
      out.put("outcome", "opened")
      invoke.resolve(out)
    } catch (e: Exception) {
      invoke.reject(e.message ?: e.toString())
    }
  }
}
