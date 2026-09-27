use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct AppSettings {
    pub auto_hide_navbar: bool,
    pub sidebar_collapsed: bool,
    pub show_hidden: bool,
    pub show_preview: bool,
    pub corner_roundness: u32,
    pub window_opacity: u32,
    pub compact_navbar: bool,
    pub is_grid_view: bool,
    pub zoom_level: i32,
    pub icon_theme: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_hide_navbar: false,
            sidebar_collapsed: false,
            show_hidden: false,
            show_preview: true,
            corner_roundness: 100, // 100% redondeado por defecto
            window_opacity: 100, // 100% opaco por defecto
            compact_navbar: false,
            is_grid_view: false,
            zoom_level: 0,
            icon_theme: String::new(),
        }
    }
}

impl AppSettings {
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("explor").join("settings.json"))
    }

    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let mut s = AppSettings::default();
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if let Some((k, v)) = trimmed.split_once(':') {
                            let k = k.trim().trim_matches('"');
                            let v = v.trim().trim_matches(',').trim();
                            match k {
                                "auto_hide_navbar" => s.auto_hide_navbar = v == "true",
                                "sidebar_collapsed" => s.sidebar_collapsed = v == "true",
                                "show_hidden" => s.show_hidden = v == "true",
                                "show_preview" => s.show_preview = v == "true",
                                "corner_roundness" => s.corner_roundness = v.parse().unwrap_or(100),
                                "window_opacity" => s.window_opacity = v.parse().unwrap_or(100),
                                "compact_navbar" => s.compact_navbar = v == "true",
                                "is_grid_view" => s.is_grid_view = v == "true",
                                "zoom_level" => s.zoom_level = v.parse().unwrap_or(0),
                                "icon_theme" => s.icon_theme = v.trim_matches('"').to_string(),
                                _ => {}
                            }
                        }
                    }
                    return s;
                }
            }
        }
        AppSettings::default()
    }

    pub fn save(&self) {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let json = format!(
                "{{\n  \"auto_hide_navbar\": {},\n  \"sidebar_collapsed\": {},\n  \"show_hidden\": {},\n  \"show_preview\": {},\n  \"corner_roundness\": {},\n  \"window_opacity\": {},\n  \"compact_navbar\": {},\n  \"is_grid_view\": {},\n  \"zoom_level\": {},\n  \"icon_theme\": \"{}\"\n}}\n",
                self.auto_hide_navbar, self.sidebar_collapsed, self.show_hidden, self.show_preview, self.corner_roundness, self.window_opacity, self.compact_navbar, self.is_grid_view, self.zoom_level, self.icon_theme
            );
            let _ = std::fs::write(path, json);
        }
    }
}

pub fn show_settings_dialog<W: IsA<gtk4::Widget>, F>(
    parent: &W,
    settings: Rc<RefCell<AppSettings>>,
    shortcuts: Rc<RefCell<crate::ui::keybindings::ShortcutsManager>>,
    on_settings_changed: F,
) where
    F: Fn(AppSettings) + 'static + Clone,
{
    let notify_change = {
        let s = settings.clone();
        let on_change = on_settings_changed;
        Rc::new(move || {
            let snapshot = { s.borrow().clone() };
            on_change(snapshot);
        })
    };

    let dialog = adw::PreferencesDialog::builder()
        .title("Explor Settings")
        .build();

    let page = adw::PreferencesPage::new();

    // 1. Interfaz y Navegación
    let nav_group = adw::PreferencesGroup::builder()
        .title("Interface & Navigation")
        .description("Display options for bars and panels")
        .build();

    let auto_hide_row = adw::SwitchRow::builder()
        .title("Auto-hide navigation bar")
        .subtitle("Show top bar only on hover (Toggle with F10)")
        .active(settings.borrow().auto_hide_navbar)
        .build();

    let sidebar_row = adw::SwitchRow::builder()
        .title("Compact sidebar (icons only)")
        .subtitle("Start with sidebar collapsed to icons only")
        .active(settings.borrow().sidebar_collapsed)
        .build();

    let preview_row = adw::SwitchRow::builder()
        .title("Show preview pane")
        .subtitle("Show details and preview of selected file (Space)")
        .active(settings.borrow().show_preview)
        .build();

    let hidden_row = adw::SwitchRow::builder()
        .title("Show hidden files")
        .subtitle("Show files starting with dot (.) by default")
        .active(settings.borrow().show_hidden)
        .build();

    let s_clone = settings.clone();
    let n = notify_change.clone();
    auto_hide_row.connect_active_notify(move |row| {
        s_clone.borrow_mut().auto_hide_navbar = row.is_active();
        s_clone.borrow().save();
        n();
    });

    let s_clone = settings.clone();
    let n = notify_change.clone();
    sidebar_row.connect_active_notify(move |row| {
        s_clone.borrow_mut().sidebar_collapsed = row.is_active();
        s_clone.borrow().save();
        n();
    });

    let s_clone = settings.clone();
    let n = notify_change.clone();
    preview_row.connect_active_notify(move |row| {
        s_clone.borrow_mut().show_preview = row.is_active();
        s_clone.borrow().save();
        n();
    });

    let s_clone = settings.clone();
    let n = notify_change.clone();
    hidden_row.connect_active_notify(move |row| {
        s_clone.borrow_mut().show_hidden = row.is_active();
        s_clone.borrow().save();
        n();
    });

    let grid_row = adw::SwitchRow::builder()
        .title("Grid view (cards)")
        .subtitle("Show files as card grid (Toggle with v)")
        .active(settings.borrow().is_grid_view)
        .build();

    let s_clone = settings.clone();
    let n = notify_change.clone();
    grid_row.connect_active_notify(move |row| {
        s_clone.borrow_mut().is_grid_view = row.is_active();
        s_clone.borrow().save();
        n();
    });

    let shortcuts_row = adw::ActionRow::builder()
        .title("Keyboard shortcuts & commands")
        .subtitle("Customize commands and shortcuts with collision prevention")
        .build();

    let btn_config_shortcuts = gtk4::Button::builder()
        .label("Customize shortcuts...")
        .icon_name("input-keyboard-symbolic")
        .valign(gtk4::Align::Center)
        .build();

    let parent_w = parent.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
    let sc_clone = shortcuts.clone();
    let n_sc = notify_change.clone();
    btn_config_shortcuts.connect_clicked(move |_| {
        if let Some(ref w) = parent_w {
            let n_inner = n_sc.clone();
            crate::ui::shortcuts_dialog::show_shortcuts_dialog(w, sc_clone.clone(), move || {
                n_inner();
            });
        }
    });

    shortcuts_row.add_suffix(&btn_config_shortcuts);

    nav_group.add(&auto_hide_row);
    nav_group.add(&sidebar_row);
    nav_group.add(&grid_row);
    nav_group.add(&preview_row);
    nav_group.add(&hidden_row);
    nav_group.add(&shortcuts_row);
    page.add(&nav_group);

    // 2. Apariencia y Bordes (Nueva sección)
    let style_group = adw::PreferencesGroup::builder()
        .title("Appearance & Borders")
        .description("Customize styling, opacity and corner roundness")
        .build();

    let radius_row = adw::ActionRow::builder()
        .title("Corner roundness")
        .subtitle("From square corners (0%) to rounded (100%)")
        .build();

    let radius_scale = gtk4::Scale::with_range(
        gtk4::Orientation::Horizontal,
        0.0,
        100.0,
        5.0,
    );
    radius_scale.set_value(settings.borrow().corner_roundness as f64);
    radius_scale.set_hexpand(true);
    radius_scale.set_width_request(200);
    radius_scale.set_draw_value(true);
    radius_scale.set_value_pos(gtk4::PositionType::Right);
    radius_scale.add_mark(0.0, gtk4::PositionType::Bottom, Some("Square"));
    radius_scale.add_mark(50.0, gtk4::PositionType::Bottom, Some("50%"));
    radius_scale.add_mark(100.0, gtk4::PositionType::Bottom, Some("Rounded"));

    let s_clone = settings.clone();
    let n = notify_change.clone();
    radius_scale.connect_value_changed(move |scale| {
        let val = scale.value().round() as u32;
        s_clone.borrow_mut().corner_roundness = val;
        s_clone.borrow().save();
        let op = s_clone.borrow().window_opacity;
        crate::ui::theme::apply_theme_css(val, op);
        n();
    });

    radius_row.add_suffix(&radius_scale);
    style_group.add(&radius_row);

    // Opción de Transparencia de la ventana
    let opacity_row = adw::ActionRow::builder()
        .title("Window opacity / transparency")
        .subtitle("Background opacity level (30% translucent to 100% opaque)")
        .build();

    let opacity_scale = gtk4::Scale::with_range(
        gtk4::Orientation::Horizontal,
        30.0,
        100.0,
        5.0,
    );
    opacity_scale.set_value(settings.borrow().window_opacity as f64);
    opacity_scale.set_hexpand(true);
    opacity_scale.set_width_request(200);
    opacity_scale.set_draw_value(true);
    opacity_scale.set_value_pos(gtk4::PositionType::Right);
    opacity_scale.add_mark(30.0, gtk4::PositionType::Bottom, Some("30%"));
    opacity_scale.add_mark(50.0, gtk4::PositionType::Bottom, Some("50%"));
    opacity_scale.add_mark(80.0, gtk4::PositionType::Bottom, Some("80%"));
    opacity_scale.add_mark(100.0, gtk4::PositionType::Bottom, Some("Opaque"));

    let s_clone = settings.clone();
    let n = notify_change.clone();
    opacity_scale.connect_value_changed(move |scale| {
        let val = scale.value().round() as u32;
        s_clone.borrow_mut().window_opacity = val;
        s_clone.borrow().save();
        let roundness = s_clone.borrow().corner_roundness;
        crate::ui::theme::apply_theme_css(roundness, val);
        n();
    });

    opacity_row.add_suffix(&opacity_scale);
    style_group.add(&opacity_row);

    let compact_nav_row = adw::SwitchRow::builder()
        .title("Compact navigation bar")
        .subtitle("Narrower layout and smaller elements in top bar")
        .active(settings.borrow().compact_navbar)
        .build();

    let s_clone = settings.clone();
    let n = notify_change.clone();
    compact_nav_row.connect_active_notify(move |row| {
        s_clone.borrow_mut().compact_navbar = row.is_active();
        s_clone.borrow().save();
        n();
    });

    style_group.add(&compact_nav_row);

    // Selector de tema de MrDemonc-SHELL
    let shell_themes = crate::ui::theme::get_available_mrdemonc_themes();
    if !shell_themes.is_empty() {
        let current_shell_theme = crate::ui::theme::get_current_theme_id().unwrap_or_else(|| "anime-sunset".to_string());
        let theme_names: Vec<String> = shell_themes.iter().map(|(_, name)| name.clone()).collect();
        let theme_name_strs: Vec<&str> = theme_names.iter().map(|s| s.as_str()).collect();
        let shell_string_list = gtk4::StringList::new(&theme_name_strs);

        let initial_shell_idx = shell_themes.iter().position(|(id, _)| id == &current_shell_theme).unwrap_or(0) as u32;

        let shell_theme_row = adw::ComboRow::builder()
            .title("MrDemonc-SHELL theme")
            .subtitle("Synchronize theme with your Linux desktop shell")
            .model(&shell_string_list)
            .selected(initial_shell_idx)
            .build();

        let st_list = shell_themes.clone();
        let n_st = notify_change.clone();
        shell_theme_row.connect_selected_notify(move |combo| {
            let idx = combo.selected() as usize;
            if let Some((selected_id, _)) = st_list.get(idx) {
                crate::ui::theme::set_mrdemonc_theme(selected_id);
                crate::ui::theme::reload_theme();
                n_st();
            }
        });

        style_group.add(&shell_theme_row);
    }

    // Selector de tema de iconos del sistema
    let available_themes = crate::ui::theme::get_available_icon_themes();
    let mut theme_options = vec!["System Default".to_string()];
    theme_options.extend(available_themes);

    let theme_strs: Vec<&str> = theme_options.iter().map(|s| s.as_str()).collect();
    let string_list = gtk4::StringList::new(&theme_strs);

    let cur_theme = settings.borrow().icon_theme.clone();
    let initial_idx = if cur_theme.is_empty() {
        0
    } else {
        theme_options.iter().position(|t| t == &cur_theme).unwrap_or(0) as u32
    };

    let icon_theme_row = adw::ComboRow::builder()
        .title("System icon theme")
        .subtitle("Change application icons using installed system themes")
        .model(&string_list)
        .selected(initial_idx)
        .build();

    let s_clone = settings.clone();
    let opts = theme_options.clone();
    let n = notify_change;
    icon_theme_row.connect_selected_notify(move |combo| {
        let idx = combo.selected() as usize;
        if let Some(selected_name) = opts.get(idx) {
            let val = if idx == 0 {
                String::new()
            } else {
                selected_name.clone()
            };
            s_clone.borrow_mut().icon_theme = val.clone();
            s_clone.borrow().save();
            crate::ui::theme::apply_icon_theme(&val);
            n();
        }
    });

    style_group.add(&icon_theme_row);
    page.add(&style_group);

    // 3. Acerca de
    let about_group = adw::PreferencesGroup::builder()
        .title("About Explor")
        .description("Ultra-fast file manager written in Rust with GTK4 and Libadwaita")
        .build();

    let ver_row = adw::ActionRow::builder()
        .title("Version")
        .subtitle("0.1.0 (Native Wayland Mode)")
        .build();

    about_group.add(&ver_row);
    page.add(&about_group);

    dialog.add(&page);
    dialog.present(Some(parent));
}
