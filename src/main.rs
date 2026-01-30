mod app;
mod config;
mod widgets;

use app::App;

fn main() -> cosmic::iced::Result {
    cosmic::app::run::<App>(cosmic::app::Settings::default(), ())
}
