//! Card #395: the phone's notifications. The notifier's cards are drawn by
//! the toast engine on a desktop; here a [`Phone`] toaster hands Talk's
//! mentions and DMs, and agents' messages to the person, to the app's
//! Kotlin `NotifyPlugin` (app/gen/android), which posts Android
//! notifications. Every other card (asks, held messages, a card waiting on
//! you) stays off the phone: there, only mentions and DMs notify.
//!
//! A tap opens Irori with the note's id; [`take_tap`] reads it when the
//! page says it listens (app:open-ready: at its start, and each time it
//! shows again), and opens that conversation.

use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};
use tauri::plugin::{Builder, PluginHandle, TauriPlugin};
use tauri::{Listener, Wry};

/// The Kotlin plugin, once the app has set it up.
pub type Slot = Arc<OnceLock<PluginHandle<Wry>>>;

#[derive(Serialize)]
struct Show<'a> {
    id: &'a str,
    title: &'a str,
    body: &'a str,
}

#[derive(Serialize)]
struct Close<'a> {
    id: &'a str,
}

#[derive(Deserialize, Default)]
struct Shown {
    #[serde(default)]
    #[allow(dead_code)]
    shown: bool,
}

#[derive(Deserialize, Default)]
struct Tap {
    #[serde(default)]
    id: String,
}

/// The notifier's toaster on the phone.
pub struct Phone(pub Slot);

impl notify::Toaster for Phone {
    fn configure(&self, _: toast::Config) {}
    fn set_options(&self, _: toast::Options) {}
    fn post(&self, n: toast::Note) {
        if !notify::on_phone(&n.id) {
            return;
        }
        let Some(h) = self.0.get() else { return };
        let r: Result<Shown, _> = h.run_mobile_plugin(
            "show",
            Show {
                id: &n.id,
                title: &n.title,
                body: &n.body,
            },
        );
        if let Err(e) = r {
            eprintln!("irori: notify: {e}");
        }
    }
    fn close(&self, id: &str) {
        if let Some(h) = self.0.get() {
            let _: Result<(), _> = h.run_mobile_plugin("close", Close { id });
        }
    }
}

/// The note the person tapped since the last call, if any.
pub fn take_tap(slot: &Slot) -> Option<notify::Target> {
    let r: Tap = slot.get()?.run_mobile_plugin("takeTap", ()).ok()?;
    notify::phone_target(&r.id)
}

/// The plugin: on setup it registers the Kotlin class into `slot`, and
/// opens a tap that launched Irori once the page listens for app:open.
pub fn init(slot: Slot, n: notify::Notifier) -> TauriPlugin<Wry> {
    Builder::new("irorinotify")
        .setup(move |app, api| {
            let h = api.register_android_plugin("com.stephenshorton.irori", "NotifyPlugin")?;
            let _ = slot.set(h);
            let slot = slot.clone();
            app.listen_any(irori_window::events::EVENT_APP_OPEN_READY, move |_| {
                // Off the main thread, where the Kotlin side answers.
                let (slot, n) = (slot.clone(), n.clone());
                std::thread::spawn(move || {
                    if let Some(t) = take_tap(&slot) {
                        n.open(t);
                    }
                });
            });
            Ok(())
        })
        .build()
}
