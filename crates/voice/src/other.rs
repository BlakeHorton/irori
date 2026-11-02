//! No engine in this build yet (macOS and Android come with #240's Mac
//! half). The window hides the mic on [`Status::Unsupported`].

use crate::{ErrorCode, Sink, StartError, Status};

const WHY: &str = "Voice input is not available on this device yet.";

pub(crate) fn status() -> Status {
    Status::Unsupported(WHY.to_string())
}

pub(crate) fn open_settings(_code: ErrorCode) -> Result<(), String> {
    Err(WHY.to_string())
}

pub(crate) struct Session;

impl Session {
    pub(crate) fn stop(self) {}
}

pub(crate) fn start(_sink: Sink) -> Result<Session, StartError> {
    Err(StartError {
        code: ErrorCode::Unsupported,
        message: WHY.to_string(),
    })
}
