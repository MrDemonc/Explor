mod fs;
mod ui;

use std::cell::RefCell;
use std::rc::Rc;
use libadwaita as adw;
use libadwaita::prelude::*;

const APP_ID: &str = "com.demonc.explor";

thread_local! {
    static ACTIVE_WINDOW: RefCell<Option<Rc<ui::MainWindow>>> = const { RefCell::new(None) };
}

fn get_or_create_window(app: &adw::Application, force_new: bool) -> Rc<ui::MainWindow> {
    if !force_new {
        let existing = ACTIVE_WINDOW.with(|cell| {
            let mut opt = cell.borrow_mut();
            if let Some(win) = opt.as_ref() {
                if win.window().is_visible() {
                    return Some(win.clone());
                }
            }
            *opt = None;
            None
        });
        if let Some(win) = existing {
            return win;
        }
    }

    load_css();
    let new_win = ui::MainWindow::new(app);
    ACTIVE_WINDOW.with(|cell| {
        *cell.borrow_mut() = Some(new_win.clone());
    });
    new_win
}

fn main() -> glib::ExitCode {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE | gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    app.connect_startup(|app| {
        load_css();
        ui::theme::setup_theme_watcher();
        setup_file_manager_dbus(app);
    });

    app.connect_activate(|app| {
        load_css();
        let win = get_or_create_window(app, false);
        win.present();
    });

    app.connect_open(|app, files, _hint| {
        load_css();
        let win = get_or_create_window(app, false);
        let mut first = true;
        for file in files {
            if let Some(path) = file.path() {
                let path_str = path.to_string_lossy().to_string();
                if first {
                    win.open_target(&path_str, false);
                    first = false;
                } else if path.is_dir() {
                    win.open_new_tab(path);
                } else {
                    win.open_target(&path_str, true);
                }
            }
        }
        win.present();
    });

    app.connect_command_line(|app, cmd_line| {
        load_css();
        let raw_args = cmd_line.arguments();
        let mut select_mode = false;
        let mut new_window = false;
        let mut target_paths = Vec::new();

        for arg in raw_args.iter().skip(1) {
            let s = arg.to_string_lossy().to_string();
            match s.as_str() {
                "--select" | "-s" => select_mode = true,
                "--new-window" | "-w" => new_window = true,
                _ if s.starts_with('-') => {} // Omitir opciones no reconocidas
                _ => target_paths.push(s),
            }
        }

        let win = get_or_create_window(app, new_window);

        if target_paths.is_empty() {
            win.present();
        } else {
            for path_str in target_paths {
                win.open_target(&path_str, select_mode);
            }
            win.present();
        }

        glib::ExitCode::SUCCESS
    });

    app.run()
}

fn setup_file_manager_dbus(app: &adw::Application) {
    let xml = r#"<node>
      <interface name="org.freedesktop.FileManager1">
        <method name="ShowFolders">
          <arg type="as" name="URIs" direction="in"/>
          <arg type="s" name="StartupId" direction="in"/>
        </method>
        <method name="ShowItems">
          <arg type="as" name="URIs" direction="in"/>
          <arg type="s" name="StartupId" direction="in"/>
        </method>
        <method name="ShowItemProperties">
          <arg type="as" name="URIs" direction="in"/>
          <arg type="s" name="StartupId" direction="in"/>
        </method>
      </interface>
    </node>"#;

    if let Ok(node_info) = gio::DBusNodeInfo::for_xml(xml) {
        if let Some(iface) = node_info.interfaces().first().cloned() {
            let app_clone = app.clone();
            gio::bus_own_name(
                gio::BusType::Session,
                "org.freedesktop.FileManager1",
                gio::BusNameOwnerFlags::ALLOW_REPLACEMENT | gio::BusNameOwnerFlags::REPLACE,
                move |conn, _name| {
                    let app_inner = app_clone.clone();
                    let _ = conn
                        .register_object("/org/freedesktop/FileManager1", &iface)
                        .method_call(move |_conn, _sender, _path, _iface, method, params, invocation| {
                            let uris: Vec<String> = params
                                .child_value(0)
                                .iter()
                                .filter_map(|c| c.str().map(|s| s.to_string()))
                                .collect();

                            let select_mode = method == "ShowItems" || method == "ShowItemProperties";
                            let win = get_or_create_window(&app_inner, false);

                            for uri in &uris {
                                win.open_target(uri, select_mode);
                            }
                            win.present();
                            invocation.return_value(None);
                        })
                        .build();
                },
                |_conn, _name| {},
                |_conn, _name| {},
            );
        }
    }
}

fn load_css() {
    ui::theme::reload_theme();
}
