// A release build is a GUI-subsystem exe, so no console flashes (surface.md
// 7.15); a debug build keeps its console so panics are visible.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> std::process::ExitCode {
    irori_app_lib::run_main()
}
