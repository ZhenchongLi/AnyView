mod app;
mod docmod_cli;
mod drop_target;
mod fidelity;
mod file_watcher;
mod find_bar;
mod image_renderer;
mod libreoffice_cli;
mod media_renderer;
mod office_renderer;
mod pdf_renderer;
mod renderer;
mod web_renderer;
mod window;

use adw::prelude::*;
use gio::ApplicationFlags;

const APP_ID: &str = "com.anyview.linux";

fn main() -> glib::ExitCode {
    let application = adw::Application::builder()
        .application_id(APP_ID)
        .flags(ApplicationFlags::HANDLES_OPEN | ApplicationFlags::NON_UNIQUE)
        .build();

    application.connect_activate(app::on_activate);
    application.connect_open(app::on_open);

    application.run()
}
