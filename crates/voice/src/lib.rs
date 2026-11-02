//! Voice input (#240): speech to text with the platform's own recognizer,
//! so there is no model to download and it works on the phone.
//!
//! The contract every engine keeps:
//! - [`status`] says whether this build and machine can listen at all.
//!   It is cheap and never prompts.
//! - [`start`] opens one listening session and calls the sink from the
//!   engine's own thread: any number of [`Event::Partial`] (the words so far
//!   for the phrase being spoken, each replacing the last), an
//!   [`Event::Final`] when a phrase is settled, then more phrases, and
//!   exactly one [`Event::Ended`] last. A failure arrives as
//!   [`Event::Error`] just before `Ended`.
//! - [`Session::stop`] (or dropping the session) ends it; the sink still
//!   gets `Ended`. Stopping twice is harmless.
//!
//! Nothing here sends text anywhere: the window puts it in the text box and
//! the person edits and sends it. Where the audio goes is the OS
//! recognizer's business (on Windows the dictation service is Microsoft's
//! online speech recognition), and the window's Settings say so.
//!
//! Engines: `win.rs` (WinRT `Windows.Media.SpeechRecognition`).
//! macOS (`SFSpeechRecognizer`) and Android (`SpeechRecognizer`) are to
//! come (#240, built on the Mac); `other.rs` reports [`Status::Unsupported`]
//! there until they land.

use std::sync::Arc;

/// Whether this machine can listen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// Ready to start (a permission prompt may still come on first use).
    Ready,
    /// This build or OS has no engine; the window hides the mic.
    Unsupported(String),
    /// The engine exists but the person or a policy turned it off; the
    /// string says which setting to change.
    Denied(String),
}

/// Why a session failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// Microphone access is off for this app.
    MicDenied,
    /// The OS speech service is off (Windows: Online speech recognition).
    SpeechDenied,
    /// No microphone, or it is in use.
    NoMic,
    /// The recognizer needs the network and could not reach it.
    Network,
    /// The system language has no dictation.
    Language,
    /// No engine in this build.
    Unsupported,
    /// Anything else.
    Other,
}

impl ErrorCode {
    /// The stable name the window gets.
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::MicDenied => "mic-denied",
            ErrorCode::SpeechDenied => "speech-denied",
            ErrorCode::NoMic => "no-mic",
            ErrorCode::Network => "network",
            ErrorCode::Language => "language",
            ErrorCode::Unsupported => "unsupported",
            ErrorCode::Other => "other",
        }
    }
}

/// One thing a session reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The phrase so far; replaces the previous partial.
    Partial(String),
    /// A settled phrase; the next partial starts a new one.
    Final(String),
    /// The session failed; `Ended` follows.
    Error { code: ErrorCode, message: String },
    /// The session is over. Always the last event.
    Ended,
}

/// Where a session's events go. Called from the engine's thread.
pub type Sink = Arc<dyn Fn(Event) + Send + Sync + 'static>;

/// A failure to start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartError {
    pub code: ErrorCode,
    pub message: String,
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for StartError {}

#[cfg(windows)]
mod win;
#[cfg(windows)]
use win as engine;

#[cfg(not(windows))]
mod other;
#[cfg(not(windows))]
use other as engine;

/// Whether this machine can listen. Cheap; never prompts.
pub fn status() -> Status {
    engine::status()
}

/// Opens the OS settings page that fixes `code` (microphone or speech
/// privacy), where the platform has one.
pub fn open_settings(code: ErrorCode) -> Result<(), String> {
    engine::open_settings(code)
}

/// Starts a listening session. Events go to `sink` until [`Event::Ended`].
pub fn start(sink: Sink) -> Result<Session, StartError> {
    engine::start(sink).map(|inner| Session { inner: Some(inner) })
}

/// One listening session. Dropping it stops it.
pub struct Session {
    inner: Option<engine::Session>,
}

impl Session {
    /// Ends the session; the sink still gets `Ended`.
    pub fn stop(mut self) {
        if let Some(s) = self.inner.take() {
            s.stop();
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some(s) = self.inner.take() {
            s.stop();
        }
    }
}
