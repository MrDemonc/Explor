mod fs;
mod ui;

use std::path::PathBuf;
use libadwaita as adw;
use libadwaita::prelude::*;
const APP_ID: &str = "com.demonc.explor";

fn main() -> glib::ExitCode {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    app.connect_startup(|_| {
        load_css();
        ui::theme::setup_theme_watcher();
    });

    app.connect_command_line(|app, cmd_line| {
        load_css();
        let main_win = ui::MainWindow::new(app);
        
        let args = cmd_line.arguments();
        if args.len() > 1 {
            let path_arg = &args[1];
            let p = PathBuf::from(path_arg);
            if p.exists() && p.is_dir() {
                main_win.navigate_to(p, false);
            }
        }

        main_win.present();
        glib::ExitCode::SUCCESS
    });

    app.run()
}

fn load_css() {
    ui::theme::reload_theme();
}
