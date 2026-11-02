//! Voice input's window face (#240): the mic button on the text boxes.
//!
//! `voice_status` says whether to show the mic, `voice_start` opens a
//! session and returns its id, `voice_stop` ends it, and the session's
//! words arrive as `voice:event` ([`VoiceEvent`]). One session at a time:
//! starting a new one ends the old (its `ended` still arrives, with its
//! own id, so a box only reacts to the session it started). Nothing is
//! sent anywhere: the box puts the text in and the person edits and sends.

use std::sync::{Arc, Mutex, Weak};

use serde::Serialize;
use specta::Type;

use crate::Window;

/// The voice:event event.
pub const EVENT_VOICE: &str = "voice:event";

/// Whether to show the mic, and why not.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VoiceStatus {
    /// ready, denied (a setting is off; pressing still tries) or
    /// unsupported (no engine here: the mic is hidden).
    pub state: String,
    /// What to do about denied or unsupported; empty when ready.
    pub reason: String,
    /// The error code whose Settings page fixes it ("speech-denied",
    /// "mic-denied"), else empty.
    pub fix: String,
}

/// One voice:event.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VoiceEvent {
    /// The session this belongs to (voice_start's answer).
    pub session: u32,
    /// partial (the phrase so far, replacing the last partial), final (a
    /// settled phrase; may be empty when it was not understood), error
    /// (code and text set; ended follows) or ended (always last).
    pub kind: String,
    pub text: String,
    /// The error code: mic-denied, speech-denied, no-mic, network,
    /// language, unsupported or other. Empty unless kind is error.
    pub code: String,
}

/// The window's one session.
#[derive(Default)]
pub(crate) struct VoiceSlot {
    inner: Mutex<SlotInner>,
}

#[derive(Default)]
struct SlotInner {
    next: u32,
    current: Option<(u32, voice::Session)>,
}

pub(crate) fn status() -> VoiceStatus {
    match voice::status() {
        voice::Status::Ready => VoiceStatus {
            state: "ready".into(),
            reason: String::new(),
            fix: String::new(),
        },
        voice::Status::Denied(why) => VoiceStatus {
            state: "denied".into(),
            reason: why,
            // The only pre-start denial an engine reports today is the
            // Windows speech privacy switch.
            fix: voice::ErrorCode::SpeechDenied.as_str().into(),
        },
        voice::Status::Unsupported(why) => VoiceStatus {
            state: "unsupported".into(),
            reason: why,
            fix: String::new(),
        },
    }
}

fn code_from(s: &str) -> Option<voice::ErrorCode> {
    use voice::ErrorCode::*;
    [
        MicDenied,
        SpeechDenied,
        NoMic,
        Network,
        Language,
        Unsupported,
        Other,
    ]
    .into_iter()
    .find(|c| c.as_str() == s)
}

impl Window {
    /// Starts a session (ending any other) and returns its id. Blocking:
    /// the engine compiles its grammar and opens the microphone first.
    pub fn voice_start(self: &Arc<Window>) -> Result<u32, String> {
        let old = {
            let mut s = self.voice.inner.lock().map_err(|e| e.to_string())?;
            s.current.take()
        };
        // Stopped outside the lock: it waits for the engine's thread.
        drop(old);
        let id = {
            let mut s = self.voice.inner.lock().map_err(|e| e.to_string())?;
            s.next = s.next.wrapping_add(1).max(1);
            s.next
        };
        let w: Weak<Window> = Arc::downgrade(self);
        let sink: voice::Sink = Arc::new(move |e| {
            if let Some(w) = w.upgrade() {
                w.emit(EVENT_VOICE, &event(id, e));
            }
        });
        let session = voice::start(sink).map_err(|e| e.message)?;
        let replaced = {
            let mut s = self.voice.inner.lock().map_err(|e| e.to_string())?;
            s.current.replace((id, session))
        };
        drop(replaced);
        Ok(id)
    }

    /// Ends `session` if it is still the current one. Blocking.
    pub fn voice_stop(&self, session: u32) -> Result<(), String> {
        let taken = {
            let mut s = self.voice.inner.lock().map_err(|e| e.to_string())?;
            match &s.current {
                Some((id, _)) if *id == session => s.current.take(),
                _ => None,
            }
        };
        if let Some((_, sess)) = taken {
            sess.stop();
        }
        Ok(())
    }

    /// Opens the OS settings page for an error code from a voice:event or
    /// the status's `fix`.
    pub fn voice_open_settings(&self, code: &str) -> Result<(), String> {
        let c = code_from(code).ok_or_else(|| format!("unknown voice error code {code:?}"))?;
        voice::open_settings(c)
    }
}

fn event(session: u32, e: voice::Event) -> VoiceEvent {
    let (kind, text, code) = match e {
        voice::Event::Partial(t) => ("partial", t, ""),
        voice::Event::Final(t) => ("final", t, ""),
        voice::Event::Error { code, message } => ("error", message, code.as_str()),
        voice::Event::Ended => ("ended", String::new(), ""),
    };
    VoiceEvent {
        session,
        kind: kind.into(),
        text,
        code: code.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_map_to_the_window_shape() {
        assert_eq!(
            event(3, voice::Event::Partial("hello wor".into())),
            VoiceEvent {
                session: 3,
                kind: "partial".into(),
                text: "hello wor".into(),
                code: "".into()
            }
        );
        assert_eq!(
            event(3, voice::Event::Final("hello world".into())).kind,
            "final"
        );
        let e = event(
            4,
            voice::Event::Error {
                code: voice::ErrorCode::SpeechDenied,
                message: "turn it on".into(),
            },
        );
        assert_eq!(
            (e.kind.as_str(), e.code.as_str(), e.text.as_str()),
            ("error", "speech-denied", "turn it on")
        );
        assert_eq!(event(4, voice::Event::Ended).kind, "ended");
    }

    #[test]
    fn codes_round_trip() {
        for c in [
            "mic-denied",
            "speech-denied",
            "no-mic",
            "network",
            "language",
            "unsupported",
            "other",
        ] {
            assert_eq!(code_from(c).map(|x| x.as_str()), Some(c));
        }
        assert_eq!(code_from("nope"), None);
    }

    #[test]
    fn status_has_a_known_state() {
        let s = status();
        assert!(["ready", "denied", "unsupported"].contains(&s.state.as_str()));
        assert_eq!(s.state == "ready", s.reason.is_empty());
    }
}
