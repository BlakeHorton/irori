//! Card #270: the Android half of the in-app update. A tiny plugin registers
//! the app's Kotlin `ApkInstallPlugin` (app/gen/android) and hands the
//! updater an installer that asks it to open Android's install screen on a
//! verified APK.

use std::path::Path;
use std::sync::Arc;

use ::update as upd;
use serde::{Deserialize, Serialize};
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

#[derive(Serialize)]
struct Args<'a> {
    path: &'a str,
}

#[derive(Deserialize)]
struct Opened {
    outcome: String,
}

/// The plugin: on setup it registers the Kotlin class and wires the
/// updater's installer to it.
pub fn init<R: Runtime>(u: Arc<upd::Updater>) -> TauriPlugin<R> {
    Builder::new("apkinstall")
        .setup(move |_app, api| {
            let h = api.register_android_plugin("com.stephenshorton.irori", "ApkInstallPlugin")?;
            u.set_apk_installer(Arc::new(move |apk: &Path| {
                let path = apk.to_string_lossy();
                let r: Opened = h
                    .run_mobile_plugin("install", Args { path: &path })
                    .map_err(|e| e.to_string())?;
                Ok(match r.outcome.as_str() {
                    "needs-permission" => upd::ApkOutcome::NeedsPermission,
                    _ => upd::ApkOutcome::Opened,
                })
            }));
            Ok(())
        })
        .build()
}
