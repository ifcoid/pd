mod app;
mod pdf_ops;

use app::App;

fn main() -> iced::Result {
    iced::application(App::boot, App::update, App::view)
        .title("pd")
        .theme(App::theme)
        .window_size([880.0, 700.0])
        .centered()
        .run()
}
