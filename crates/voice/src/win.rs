//! The Windows engine: WinRT `Windows.Media.SpeechRecognition`, continuous
//! dictation (the `Dictation` topic constraint). `HypothesisGenerated`
//! gives the partials, the continuous session's `ResultGenerated` the
//! finals and `Completed` the end.
//!
//! Each session owns one thread: the recognizer is created, started,
//! stopped and closed there, and the event handlers (which WinRT calls on
//! its own pool threads) only call the sink. windows-core joins the MTA
//! itself when the thread has no apartment yet.
//!
//! The dictation topic uses Microsoft's online speech service, so it needs
//! Settings > Privacy & security > Speech > Online speech recognition on
//! (else `SPERR_SPEECH_PRIVACY_POLICY_NOT_ACCEPTED`), and microphone access
//! for desktop apps (else `E_ACCESSDENIED`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use windows::Foundation::TypedEventHandler;
use windows::Media::SpeechRecognition::{
    ISpeechRecognitionConstraint, SpeechContinuousRecognitionCompletedEventArgs,
    SpeechContinuousRecognitionResultGeneratedEventArgs, SpeechContinuousRecognitionSession,
    SpeechRecognitionConfidence, SpeechRecognitionHypothesisGeneratedEventArgs,
    SpeechRecognitionResultStatus, SpeechRecognitionScenario, SpeechRecognitionTopicConstraint,
    SpeechRecognizer,
};
use windows::core::{HRESULT, HSTRING, Interface};

use crate::{ErrorCode, Event, Sink, StartError, Status};

/// SPERR_SPEECH_PRIVACY_POLICY_NOT_ACCEPTED: Online speech recognition is off.
const SPEECH_PRIVACY: HRESULT = HRESULT(0x8004_5509_u32 as i32);
/// E_ACCESSDENIED: microphone access is off for desktop apps.
const ACCESS_DENIED: HRESULT = HRESULT(0x8007_0005_u32 as i32);
/// SPERR_NOT_FOUND / no capture device (seen as 0x8004503A on machines without a mic).
const NO_AUDIO_DEVICE: HRESULT = HRESULT(0x8004_503A_u32 as i32);

pub(crate) const SPEECH_SETTING: &str = "Turn on Online speech recognition in Windows Settings > Privacy & security > Speech, then try again.";
pub(crate) const MIC_SETTING: &str = "Allow microphone access for desktop apps in Windows Settings > Privacy & security > Microphone, then try again.";

pub(crate) fn status() -> Status {
    // The class is there on every Windows 10/11; whether the person allowed
    // it only shows when a session starts (no API reads the privacy switch
    // without starting one).
    match SpeechRecognizer::SystemSpeechLanguage() {
        Ok(_) => Status::Ready,
        Err(e) => Status::Unsupported(format!("Windows speech recognition is not available ({e})")),
    }
}

pub(crate) struct Session {
    stop: mpsc::Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl Session {
    pub(crate) fn stop(mut self) {
        let _ = self.stop.send(());
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

pub(crate) fn start(sink: Sink) -> Result<Session, StartError> {
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<(), StartError>>(1);
    let done_tx = stop_tx.clone();
    let thread = std::thread::Builder::new()
        .name("irori-voice".into())
        .spawn(move || run(sink, stop_rx, done_tx, ready_tx))
        .map_err(|e| StartError {
            code: ErrorCode::Other,
            message: e.to_string(),
        })?;
    match ready_rx.recv() {
        Ok(Ok(())) => Ok(Session {
            stop: stop_tx,
            thread: Some(thread),
        }),
        Ok(Err(e)) => {
            let _ = thread.join();
            Err(e)
        }
        Err(_) => {
            let _ = thread.join();
            Err(StartError {
                code: ErrorCode::Other,
                message: "the speech thread ended".into(),
            })
        }
    }
}

/// The session thread. Reports the start through `ready`, then waits for a
/// stop (the window) or the session's own end (`done`, from Completed).
fn run(
    sink: Sink,
    stop: mpsc::Receiver<()>,
    done: mpsc::Sender<()>,
    ready: mpsc::SyncSender<Result<(), StartError>>,
) {
    let failure: Arc<Mutex<Option<(ErrorCode, String)>>> = Arc::new(Mutex::new(None));
    let opened = match open(&sink, done, failure.clone()) {
        Ok(o) => o,
        Err(e) => {
            let _ = ready.send(Err(e));
            return;
        }
    };
    let _ = ready.send(Ok(()));
    let _ = stop.recv();
    let (recognizer, session) = opened;
    // StopAsync fails once the session already ended on its own; either way
    // it is over.
    if let Ok(op) = session.StopAsync() {
        let _ = op.join();
    }
    let _ = recognizer.Close();
    if let Some((code, message)) = failure.lock().ok().and_then(|mut f| f.take()) {
        sink(Event::Error { code, message });
    }
    sink(Event::Ended);
}

fn open(
    sink: &Sink,
    done: mpsc::Sender<()>,
    failure: Arc<Mutex<Option<(ErrorCode, String)>>>,
) -> Result<(SpeechRecognizer, SpeechContinuousRecognitionSession), StartError> {
    let recognizer = SpeechRecognizer::new().map_err(start_error)?;
    let topic = SpeechRecognitionTopicConstraint::Create(
        SpeechRecognitionScenario::Dictation,
        &HSTRING::from("dictation"),
    )
    .map_err(start_error)?;
    recognizer
        .Constraints()
        .and_then(|c| c.Append(&topic.cast::<ISpeechRecognitionConstraint>()?))
        .map_err(start_error)?;
    let compiled = recognizer
        .CompileConstraintsAsync()
        .and_then(|op| op.join())
        .map_err(start_error)?;
    let st = compiled.Status().map_err(start_error)?;
    if st != SpeechRecognitionResultStatus::Success {
        let (code, message) = status_error(st);
        return Err(StartError { code, message });
    }

    let s = sink.clone();
    recognizer
        .HypothesisGenerated(&TypedEventHandler::<
            SpeechRecognizer,
            SpeechRecognitionHypothesisGeneratedEventArgs,
        >::new(move |_, args| {
            if let Some(args) = args.as_ref() {
                let text = args.Hypothesis()?.Text()?.to_string();
                if !text.is_empty() {
                    s(Event::Partial(text));
                }
            }
            Ok(())
        }))
        .map_err(start_error)?;

    let session = recognizer
        .ContinuousRecognitionSession()
        .map_err(start_error)?;
    let s = sink.clone();
    session
        .ResultGenerated(&TypedEventHandler::<
            SpeechContinuousRecognitionSession,
            SpeechContinuousRecognitionResultGeneratedEventArgs,
        >::new(move |_, args| {
            if let Some(args) = args.as_ref() {
                let r = args.Result()?;
                let ok = r.Status()? == SpeechRecognitionResultStatus::Success
                    && r.Confidence()? != SpeechRecognitionConfidence::Rejected;
                let text = r.Text()?.to_string();
                // A rejected or empty phrase still ends the partial.
                s(Event::Final(if ok { text } else { String::new() }));
            }
            Ok(())
        }))
        .map_err(start_error)?;

    let ended = Arc::new(AtomicBool::new(false));
    session
        .Completed(&TypedEventHandler::<
            SpeechContinuousRecognitionSession,
            SpeechContinuousRecognitionCompletedEventArgs,
        >::new(move |_, args| {
            if ended.swap(true, Ordering::SeqCst) {
                return Ok(());
            }
            if let Some(args) = args.as_ref() {
                let st = args.Status()?;
                if st != SpeechRecognitionResultStatus::Success
                    && st != SpeechRecognitionResultStatus::UserCanceled
                {
                    if let Ok(mut f) = failure.lock() {
                        *f = Some(status_error(st));
                    }
                }
            }
            let _ = done.send(());
            Ok(())
        }))
        .map_err(start_error)?;

    session
        .StartAsync()
        .and_then(|op| op.join())
        .map_err(start_error)?;
    Ok((recognizer, session))
}

fn start_error(e: windows::core::Error) -> StartError {
    let (code, message) = hresult_error(&e);
    StartError { code, message }
}

/// The window's message for a failed WinRT call.
pub(crate) fn hresult_error(e: &windows::core::Error) -> (ErrorCode, String) {
    match e.code() {
        SPEECH_PRIVACY => (ErrorCode::SpeechDenied, SPEECH_SETTING.to_string()),
        ACCESS_DENIED => (ErrorCode::MicDenied, MIC_SETTING.to_string()),
        NO_AUDIO_DEVICE => (ErrorCode::NoMic, "No microphone was found.".to_string()),
        c => (
            ErrorCode::Other,
            format!(
                "Speech recognition failed: {} (0x{:08X})",
                e.message(),
                c.0 as u32
            ),
        ),
    }
}

/// The window's message for a session that ended badly.
pub(crate) fn status_error(st: SpeechRecognitionResultStatus) -> (ErrorCode, String) {
    match st {
        SpeechRecognitionResultStatus::MicrophoneUnavailable => (
            ErrorCode::NoMic,
            "The microphone is unavailable (none connected, or in use).".into(),
        ),
        SpeechRecognitionResultStatus::NetworkFailure => (
            ErrorCode::Network,
            "Windows speech recognition could not reach its service.".into(),
        ),
        SpeechRecognitionResultStatus::TopicLanguageNotSupported => (
            ErrorCode::Language,
            "Dictation is not available for the Windows speech language.".into(),
        ),
        SpeechRecognitionResultStatus::TimeoutExceeded
        | SpeechRecognitionResultStatus::PauseLimitExceeded => (
            ErrorCode::Other,
            "Listening stopped after a long silence.".into(),
        ),
        SpeechRecognitionResultStatus::AudioQualityFailure => (
            ErrorCode::Other,
            "The audio was too poor to recognise.".into(),
        ),
        other => (
            ErrorCode::Other,
            format!("Speech recognition stopped (status {}).", other.0),
        ),
    }
}
