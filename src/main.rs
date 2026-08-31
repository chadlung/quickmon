use quickmon::app;

fn main() -> iced::Result {
    iced::application(app::boot, app::update, app::view)
        .title("QuickMon")
        .run()
}
