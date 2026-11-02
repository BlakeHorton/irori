//! Windows glue: the Settings page for a denied or missing microphone.

use crate::ErrorCode;

/// Opens Settings > Privacy & security > Microphone.
pub(crate) fn open_settings(code: ErrorCode) -> Result<(), String> {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use windows::core::w;
    if !matches!(code, ErrorCode::MicDenied | ErrorCode::NoMic) {
        return Err("no settings page for this".into());
    }
    // ShellExecute returns a value above 32 on success.
    let h = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            w!("ms-settings:privacy-microphone"),
            None,
            None,
            SW_SHOWNORMAL,
        )
    };
    if h.0 as isize > 32 {
        Ok(())
    } else {
        Err(format!("could not open Settings ({})", h.0 as isize))
    }
}
