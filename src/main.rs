// No console window on Windows release builds. Rust targets the console
// subsystem by default, so the OS allocates a terminal alongside the GUI
// whenever the binary runs. Gated on `not(debug_assertions)` deliberately:
// debug builds keep the console so panics and println! still surface while
// developing. Has no effect on any other platform.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use quickmon::app;

fn main() -> iced::Result {
    iced::application(app::boot, app::update, app::view)
        .title("QuickMon")
        .run()
}
