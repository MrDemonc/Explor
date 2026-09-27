use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use gio::prelude::*;
use libadwaita as adw;

thread_local! {
    static THEME_PROVIDER: RefCell<Option<gtk4::CssProvider>> = const { RefCell::new(None) };
    static FILE_MONITORS: RefCell<Vec<gio::FileMonitor>> = const { RefCell::new(Vec::new()) };
    static LAST_THEME_ID: RefCell<Option<String>> = const { RefCell::new(None) };
}

static WATCHER_INITIALIZED: AtomicBool = AtomicBool::new(false);

fn default_true() -> bool {
    true
}
fn default_bg() -> String {
    "#1a1d24".to_string()
}
fn default_bg_surface() -> String {
    "#14161d".to_string()
}
fn default_bg_hover() -> String {
    "#282d38".to_string()
}
fn default_border() -> String {
    "#353b49".to_string()
}
fn default_text() -> String {
    "#eceff4".to_string()
}
fn default_subtext() -> String {
    "#d8dee9".to_string()
}
fn default_primary() -> String {
    "#88c0d0".to_string()
}
fn default_success() -> String {
    "#a3be8c".to_string()
}
fn default_warning() -> String {
    "#ebcb8b".to_string()
}
fn default_danger() -> String {
    "#bf616a".to_string()
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct MrDemoncTheme {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(rename = "isDark", default = "default_true")]
    pub is_dark: bool,
    #[serde(default)]
    pub wallpaper: Option<String>,
    #[serde(default)]
    pub wallpaper_path: Option<String>,
    #[serde(default = "default_bg")]
    pub bg: String,
    #[serde(rename = "bgSurface", default = "default_bg_surface")]
    pub bg_surface: String,
    #[serde(rename = "bgHover", default = "default_bg_hover")]
    pub bg_hover: String,
    #[serde(default = "default_border")]
    pub border: String,
    #[serde(default = "default_text")]
    pub text: String,
    #[serde(default = "default_subtext")]
    pub subtext: String,
    #[serde(default)]
    pub overlay: Option<String>,
    #[serde(default = "default_primary")]
    pub primary: String,
    #[serde(default = "default_success")]
    pub success: String,
    #[serde(default = "default_warning")]
    pub warning: String,
    #[serde(default = "default_danger")]
    pub danger: String,
    #[serde(default)]
    pub cyan: Option<String>,
    #[serde(default)]
    pub pink: Option<String>,
}

pub fn get_theme_search_dirs() -> Vec<PathBuf> {
    let mut dirs_list = Vec::new();
    if let Some(home) = dirs::home_dir() {
        dirs_list.push(home.join(".config/quickshell/themes"));
        dirs_list.push(home.join("Documentos/github/MrDemonc-SHELL/themes"));
        dirs_list.push(home.join("Documentos/MrDemonc-SHELL/themes"));
    }
    dirs_list.push(PathBuf::from("/usr/share/mrdemonc-shell/themes"));
    dirs_list
}

pub fn get_builtin_theme(id: &str) -> Option<MrDemoncTheme> {
    match id {
        "anime-sunset" => Some(MrDemoncTheme {
            id: "anime-sunset".to_string(),
            name: "Anime Sunset".to_string(),
            description: "Tema artístico inspirado en cielos de anime al atardecer, nubes iluminadas y horizontes crepusculares".to_string(),
            author: "MrDemonc".to_string(),
            is_dark: true,
            wallpaper: Some("wallpaper.jpg".to_string()),
            wallpaper_path: None,
            bg: "#191519".to_string(),
            bg_surface: "#231e23".to_string(),
            bg_hover: "#322b32".to_string(),
            border: "#483d47".to_string(),
            text: "#faede8".to_string(),
            subtext: "#d1bbb4".to_string(),
            overlay: Some("#7e6c77".to_string()),
            primary: "#f0997c".to_string(),
            success: "#8ab896".to_string(),
            warning: "#e5b567".to_string(),
            danger: "#df5d52".to_string(),
            cyan: Some("#68a9b8".to_string()),
            pink: Some("#d687a8".to_string()),
        }),
        "noir" => Some(MrDemoncTheme {
            id: "noir".to_string(),
            name: "Noir".to_string(),
            description: "Tema oscuro monocromático en blanco y negro puro, con tonos grafito y platas".to_string(),
            author: "MrDemonc".to_string(),
            is_dark: true,
            wallpaper: Some("wallpaper.jpg".to_string()),
            wallpaper_path: None,
            bg: "#0f1113".to_string(),
            bg_surface: "#181a1d".to_string(),
            bg_hover: "#24272c".to_string(),
            border: "#393e46".to_string(),
            text: "#f4f5f6".to_string(),
            subtext: "#b8bec6".to_string(),
            overlay: Some("#6f7682".to_string()),
            primary: "#e2e6eb".to_string(),
            success: "#88c999".to_string(),
            warning: "#e5c07b".to_string(),
            danger: "#e06c75".to_string(),
            cyan: Some("#98c5e0".to_string()),
            pink: Some("#c8a8d8".to_string()),
        }),
        "mountains" => Some(MrDemoncTheme {
            id: "mountains".to_string(),
            name: "Mountains".to_string(),
            description: "Tema alpino inspirado en picos montañosos con niebla glaciar y bosques de coníferas".to_string(),
            author: "MrDemonc".to_string(),
            is_dark: true,
            wallpaper: Some("wallpaper.jpg".to_string()),
            wallpaper_path: None,
            bg: "#141a1a".to_string(),
            bg_surface: "#1a2222".to_string(),
            bg_hover: "#253231".to_string(),
            border: "#354746".to_string(),
            text: "#edf5f5".to_string(),
            subtext: "#aec4c3".to_string(),
            overlay: Some("#667f7e".to_string()),
            primary: "#5db1c7".to_string(),
            success: "#7eb886".to_string(),
            warning: "#e8b356".to_string(),
            danger: "#df5c68".to_string(),
            cyan: Some("#7ad7ea".to_string()),
            pink: Some("#c589a8".to_string()),
        }),
        "street" => Some(MrDemoncTheme {
            id: "street".to_string(),
            name: "Street".to_string(),
            description: "Tema atmosférico extraído de la carretera húmeda entre pinares con líneas doradas".to_string(),
            author: "MrDemonc".to_string(),
            is_dark: true,
            wallpaper: Some("wallpaper.jpg".to_string()),
            wallpaper_path: None,
            bg: "#14171a".to_string(),
            bg_surface: "#1b1f23".to_string(),
            bg_hover: "#282e35".to_string(),
            border: "#3a434c".to_string(),
            text: "#ece5de".to_string(),
            subtext: "#b8ab9f".to_string(),
            overlay: Some("#707a84".to_string()),
            primary: "#f5af19".to_string(),
            success: "#6f9479".to_string(),
            warning: "#e59b1f".to_string(),
            danger: "#c85a42".to_string(),
            cyan: Some("#77a2b2".to_string()),
            pink: Some("#b67d8f".to_string()),
        }),
        "default" => Some(MrDemoncTheme {
            id: "default".to_string(),
            name: "Default".to_string(),
            description: "Tema predeterminado inspirado en La Gran Ola de Kanagawa y tonos pizarra nórdicos".to_string(),
            author: "MrDemonc".to_string(),
            is_dark: true,
            wallpaper: Some("wallpaper.jpg".to_string()),
            wallpaper_path: None,
            bg: "#1a1d24".to_string(),
            bg_surface: "#14161d".to_string(),
            bg_hover: "#282d38".to_string(),
            border: "#353b49".to_string(),
            text: "#eceff4".to_string(),
            subtext: "#d8dee9".to_string(),
            overlay: Some("#7b889b".to_string()),
            primary: "#88c0d0".to_string(),
            success: "#a3be8c".to_string(),
            warning: "#ebcb8b".to_string(),
            danger: "#bf616a".to_string(),
            cyan: Some("#81a1c1".to_string()),
            pink: Some("#b48ead".to_string()),
        }),
        _ => None,
    }
}

pub fn get_current_theme_id() -> Option<String> {
    if let Some(home) = dirs::home_dir() {
        let config_file = home.join(".config/quickshell/current_theme.json");
        if config_file.exists() {
            if let Ok(content) = fs::read_to_string(&config_file) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(t) = v.get("theme").and_then(|s| s.as_str()) {
                        return Some(t.to_string());
                    }
                    if let Some(t) = v.get("id").and_then(|s| s.as_str()) {
                        return Some(t.to_string());
                    }
                }
            }
        }
    }
    None
}

pub fn load_theme_by_id(theme_id: &str) -> Option<MrDemoncTheme> {
    for base in get_theme_search_dirs() {
        let path1 = base.join(theme_id).join("theme.json");
        if path1.exists() {
            if let Ok(content) = fs::read_to_string(&path1) {
                if let Ok(mut th) = serde_json::from_str::<MrDemoncTheme>(&content) {
                    if th.id.is_empty() {
                        th.id = theme_id.to_string();
                    }
                    return Some(th);
                }
            }
        }
        let path2 = base.join(format!("{theme_id}.json"));
        if path2.exists() {
            if let Ok(content) = fs::read_to_string(&path2) {
                if let Ok(mut th) = serde_json::from_str::<MrDemoncTheme>(&content) {
                    if th.id.is_empty() {
                        th.id = theme_id.to_string();
                    }
                    return Some(th);
                }
            }
        }
    }
    get_builtin_theme(theme_id)
}

pub fn load_current_mrdemonc_theme() -> Option<MrDemoncTheme> {
    let theme_id = get_current_theme_id().unwrap_or_else(|| "anime-sunset".to_string());
    load_theme_by_id(&theme_id)
}

pub fn get_available_mrdemonc_themes() -> Vec<(String, String)> {
    let mut map = std::collections::BTreeMap::new();

    for base in get_theme_search_dirs() {
        if let Ok(entries) = fs::read_dir(&base) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let tj = path.join("theme.json");
                    if tj.exists() {
                        if let Ok(c) = fs::read_to_string(&tj) {
                            if let Ok(th) = serde_json::from_str::<MrDemoncTheme>(&c) {
                                let id = if !th.id.is_empty() {
                                    th.id
                                } else {
                                    path.file_name().unwrap_or_default().to_string_lossy().to_string()
                                };
                                let name = if !th.name.is_empty() { th.name } else { id.clone() };
                                map.insert(id, name);
                            }
                        }
                    }
                } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if stem != "current_theme" {
                            if let Ok(c) = fs::read_to_string(&path) {
                                if let Ok(th) = serde_json::from_str::<MrDemoncTheme>(&c) {
                                    let id = if !th.id.is_empty() { th.id } else { stem.to_string() };
                                    let name = if !th.name.is_empty() { th.name } else { id.clone() };
                                    map.insert(id, name);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    for b in ["default", "street", "mountains", "noir", "anime-sunset"] {
        if !map.contains_key(b) {
            if let Some(th) = get_builtin_theme(b) {
                map.insert(th.id, th.name);
            }
        }
    }

    map.into_iter().collect()
}

pub fn set_mrdemonc_theme(theme_id: &str) -> bool {
    // 1. Probar binario wrapper shell-theme
    if let Ok(status) = Command::new("shell-theme").args(["set", theme_id]).status() {
        if status.success() {
            return true;
        }
    }
    // 2. Probar ~/.local/bin/shell-theme
    if let Some(home) = dirs::home_dir() {
        let bin = home.join(".local/bin/shell-theme");
        if bin.exists() {
            if let Ok(status) = Command::new(&bin).args(["set", theme_id]).status() {
                if status.success() {
                    return true;
                }
            }
        }
        // 3. Probar script en carpetas de repositorio conocidas
        let candidates = [
            home.join("Documentos/github/MrDemonc-SHELL/scripts/theme_manager.py"),
            home.join("Documentos/MrDemonc-SHELL/scripts/theme_manager.py"),
            PathBuf::from("/usr/share/mrdemonc-shell/scripts/theme_manager.py"),
        ];
        for script in candidates {
            if script.exists() {
                if let Ok(status) = Command::new("python3").arg(&script).args(["set", theme_id]).status() {
                    if status.success() {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn ensure_hex(c: &str, fallback: &str) -> String {
    let t = c.trim();
    if t.is_empty() {
        return fallback.to_string();
    }
    if t.starts_with('#') {
        t.to_string()
    } else {
        format!("#{t}")
    }
}

fn hex_luminance(hex: &str) -> f64 {
    let clean = hex.trim().trim_start_matches('#');
    if clean.len() < 6 {
        return 0.5;
    }
    let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(128) as f64 / 255.0;
    let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(128) as f64 / 255.0;
    let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(128) as f64 / 255.0;

    let chan = |v: f64| -> f64 {
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };

    0.2126 * chan(r) + 0.7152 * chan(g) + 0.0722 * chan(b)
}

pub fn theme_to_gtk_css(theme: &MrDemoncTheme) -> String {
    let bg = ensure_hex(&theme.bg, "#191519");
    let bg_surface = ensure_hex(&theme.bg_surface, "#231e23");
    let bg_hover = ensure_hex(&theme.bg_hover, "#322b32");
    let border = ensure_hex(&theme.border, "#483d47");
    let text = ensure_hex(&theme.text, "#faede8");
    let subtext = ensure_hex(&theme.subtext, "#d1bbb4");
    let overlay = theme.overlay.as_deref().unwrap_or("#7e6c77");
    let overlay = ensure_hex(overlay, "#7e6c77");
    let primary = ensure_hex(&theme.primary, "#f0997c");
    let success = ensure_hex(&theme.success, "#8ab896");
    let warning = ensure_hex(&theme.warning, "#e5b567");
    let danger = ensure_hex(&theme.danger, "#df5d52");

    let lum = hex_luminance(&primary);
    let accent_fg = if lum < 0.45 { "#ffffff".to_string() } else { bg.clone() };
    let card_bg = if theme.is_dark { "rgba(255, 255, 255, 0.05)" } else { "rgba(0, 0, 0, 0.04)" };
    let shade_alpha = if theme.is_dark { "0.25" } else { "0.12" };

    format!("
        /* Colores adaptados del tema MrDemonc-SHELL: {} ({}) */
        @define-color window_bg_color {bg};
        @define-color window_fg_color {text};
        @define-color view_bg_color {bg};
        @define-color view_fg_color {text};
        @define-color background {bg};
        @define-color foreground {text};
        @define-color headerbar_bg_color {bg};
        @define-color headerbar_fg_color {text};
        @define-color headerbar_backdrop_color {bg_surface};
        @define-color headerbar_shade_color rgba(0, 0, 0, 0.15);
        @define-color headerbar_darker_shade_color rgba(0, 0, 0, 0.35);
        @define-color sidebar_bg_color {bg_surface};
        @define-color sidebar_fg_color {text};
        @define-color sidebar_backdrop_color {bg_surface};
        @define-color sidebar_shade_color rgba(0, 0, 0, 0.10);
        @define-color sidebar_border_color {border};
        @define-color secondary_sidebar_bg_color {bg_surface};
        @define-color secondary_sidebar_fg_color {text};
        @define-color card_bg_color {card_bg};
        @define-color card_fg_color {text};
        @define-color card_shade_color rgba(0, 0, 0, 0.15);
        @define-color dialog_bg_color {bg_surface};
        @define-color dialog_fg_color {text};
        @define-color popover_bg_color {bg_surface};
        @define-color popover_fg_color {text};
        @define-color thumbnail_bg_color {bg_surface};
        @define-color thumbnail_fg_color {subtext};
        @define-color borders {border};
        @define-color border_color {border};
        @define-color accent_color {primary};
        @define-color accent_bg_color {primary};
        @define-color accent_fg_color {accent_fg};
        @define-color destructive_color {danger};
        @define-color destructive_bg_color {danger};
        @define-color destructive_fg_color #ffffff;
        @define-color red {danger};
        @define-color success_color {success};
        @define-color success_bg_color {success};
        @define-color success_fg_color #ffffff;
        @define-color warning_color {warning};
        @define-color warning_bg_color {warning};
        @define-color warning_fg_color {bg};
        @define-color error_color {danger};
        @define-color error_bg_color {danger};
        @define-color error_fg_color #ffffff;
        @define-color subtext {subtext};
        @define-color overlay {overlay};
        @define-color bg_hover {bg_hover};
        @define-color shade_color rgba(0, 0, 0, {shade_alpha});
        @define-color scrollbar_outline_color rgba(255, 255, 255, 0.10);
    ",
        theme.name, theme.id
    )
}

pub fn get_system_theme_css_and_dark() -> (String, bool) {
    // 1. Prioridad: Tema actual activo de MrDemonc-SHELL
    if let Some(theme) = load_current_mrdemonc_theme() {
        let is_dark = theme.is_dark;
        let css = theme_to_gtk_css(&theme);
        return (css, is_dark);
    }

    // 2. Fallback: Configuración GTK4 generada por el shell en ~/.config/gtk-4.0/gtk.css
    if let Some(home) = dirs::home_dir() {
        let gtk4_css = home.join(".config/gtk-4.0/gtk.css");
        if gtk4_css.exists() {
            if let Ok(content) = fs::read_to_string(&gtk4_css) {
                let mut css = String::new();
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("@define-color") {
                        css.push_str(trimmed);
                        css.push('\n');
                    }
                }
                if !css.is_empty() {
                    if !css.contains("borders") {
                        css.push_str("@define-color borders rgba(255, 255, 255, 0.10);\n");
                    }
                    if !css.contains("border_color") {
                        css.push_str("@define-color border_color rgba(255, 255, 255, 0.10);\n");
                    }
                    if !css.contains("background") {
                        css.push_str("@define-color background @window_bg_color;\n");
                    }
                    if !css.contains("foreground") {
                        css.push_str("@define-color foreground @window_fg_color;\n");
                    }
                    if !css.contains("red") {
                        css.push_str("@define-color red @destructive_color;\n");
                    }
                    return (css, true);
                }
            }
        }
    }

    // 3. Fallback final: Paleta oscura predeterminada elegante
    (
        "
        @define-color background #191519;
        @define-color foreground #faede8;
        @define-color window_bg_color #191519;
        @define-color window_fg_color #faede8;
        @define-color view_bg_color #191519;
        @define-color view_fg_color #faede8;
        @define-color headerbar_bg_color #191519;
        @define-color headerbar_fg_color #faede8;
        @define-color headerbar_backdrop_color #231e23;
        @define-color headerbar_shade_color rgba(0, 0, 0, 0.15);
        @define-color headerbar_darker_shade_color rgba(0, 0, 0, 0.35);
        @define-color sidebar_bg_color #231e23;
        @define-color sidebar_fg_color #faede8;
        @define-color sidebar_backdrop_color #231e23;
        @define-color sidebar_shade_color rgba(0, 0, 0, 0.10);
        @define-color sidebar_border_color #483d47;
        @define-color secondary_sidebar_bg_color #231e23;
        @define-color secondary_sidebar_fg_color #faede8;
        @define-color card_bg_color rgba(255, 255, 255, 0.05);
        @define-color card_fg_color #faede8;
        @define-color card_shade_color rgba(0, 0, 0, 0.15);
        @define-color dialog_bg_color #231e23;
        @define-color dialog_fg_color #faede8;
        @define-color popover_bg_color #231e23;
        @define-color popover_fg_color #faede8;
        @define-color thumbnail_bg_color #231e23;
        @define-color thumbnail_fg_color #d1bbb4;
        @define-color borders #483d47;
        @define-color border_color #483d47;
        @define-color accent_color #f0997c;
        @define-color accent_bg_color #f0997c;
        @define-color accent_fg_color #ffffff;
        @define-color destructive_color #df5d52;
        @define-color destructive_bg_color #df5d52;
        @define-color destructive_fg_color #ffffff;
        @define-color red #df5d52;
        @define-color success_color #8ab896;
        @define-color success_bg_color #8ab896;
        @define-color success_fg_color #ffffff;
        @define-color warning_color #e5b567;
        @define-color warning_bg_color #e5b567;
        @define-color warning_fg_color #191519;
        @define-color error_color #df5d52;
        @define-color error_bg_color #df5d52;
        @define-color error_fg_color #ffffff;
        @define-color subtext #d1bbb4;
        @define-color overlay #7e6c77;
        @define-color bg_hover #322b32;
        @define-color shade_color rgba(0, 0, 0, 0.25);
        @define-color scrollbar_outline_color rgba(255, 255, 255, 0.10);
        ".to_string(),
        true,
    )
}

#[allow(dead_code)]
pub fn get_system_theme_css() -> String {
    get_system_theme_css_and_dark().0
}

pub fn generate_radius_css(roundness_percent: u32) -> String {
    let factor = (roundness_percent as f64 / 100.0).clamp(0.0, 1.0);

    let file_row = (12.0 * factor).round() as u32;
    let side_row = (10.0 * factor).round() as u32;
    let disk_card = (14.0 * factor).round() as u32;
    let progress = (6.0 * factor).round() as u32;
    let preview_card = (16.0 * factor).round() as u32;
    let preview_view = (12.0 * factor).round() as u32;
    let preview_scroll = (14.0 * factor).round() as u32;
    let palette_card = (22.0 * factor).round() as u32;
    let palette_entry = (14.0 * factor).round() as u32;
    let path_pill = (20.0 * factor).round() as u32;
    let action_bubble = (30.0 * factor).round() as u32;
    let bubble_btn = (20.0 * factor).round() as u32;
    let badge = (8.0 * factor).round() as u32;
    let window_radius = (20.0 * factor).round() as u32;
    let button_entry = (10.0 * factor).round() as u32;
    let context_menu_popover = (14.0 * factor).round() as u32;
    let context_menu_btn = (10.0 * factor).round() as u32;

    format!("
        .file-list row {{ border-radius: {file_row}px; }}
        .sidebar row {{ border-radius: {side_row}px; }}
        .sidebar-disk-card {{ border-radius: {disk_card}px; }}
        .disk-progress progress, .disk-progress trough {{ border-radius: {progress}px; }}
        .preview-meta-card {{ border-radius: {preview_card}px; }}
        .preview-text-view {{ border-radius: {preview_view}px; }}
        .preview-scroll {{ border-radius: {preview_scroll}px; }}
        .preview-picture {{ border-radius: {preview_scroll}px; }}
        .command-palette-card {{ border-radius: {palette_card}px; }}
        .command-palette-entry {{ border-radius: {palette_entry}px; }}
        .path-pill {{ border-radius: {path_pill}px; }}
        .action-bubble, .tabs-bubble {{ border-radius: {action_bubble}px; }}
        .tab-wrapper {{ border-radius: {bubble_btn}px; }}
        .bubble-btn {{ border-radius: {bubble_btn}px; }}
        .bubble-shortcut-badge {{ border-radius: {badge}px; }}
        window.dialog, window.background, window,
        dialog, dialog.preferences, dialog sheet,
        floating-sheet > sheet, bottom-sheet > sheet,
        dialog-host > dialog sheet, sheet {{
            border-radius: {window_radius}px;
        }}
        button, entry {{ border-radius: {button_entry}px; }}
        popover contents, popover.menu contents, .context-menu-popover contents {{
            border-radius: {context_menu_popover}px;
        }}
        .context-menu-box {{
            border-radius: {context_menu_popover}px;
        }}
        .menu-item-btn {{
            border-radius: {context_menu_btn}px;
        }}
        .card, list.boxed-list, list.content, list.boxed-list-separate > row,
        flowboxchild, .file-grid-card {{
            border-radius: {preview_card}px;
        }}
        list.boxed-list > row:first-child {{
            border-top-left-radius: {preview_card}px;
            border-top-right-radius: {preview_card}px;
        }}
        list.boxed-list > row:last-child {{
            border-bottom-left-radius: {preview_card}px;
            border-bottom-right-radius: {preview_card}px;
        }}
    ")
}

pub fn generate_opacity_css(opacity_percent: u32) -> String {
    let alpha = (opacity_percent.clamp(20, 100) as f64) / 100.0;
    if opacity_percent >= 100 {
        String::new()
    } else {
        let side_alpha = (alpha * 0.95).clamp(0.15, 1.0);
        let header_alpha = (alpha * 0.90).clamp(0.15, 1.0);
        let dialog_alpha = (alpha * 0.96).clamp(0.20, 1.0);
        format!("
            window, window.background, .background,
            split-view, paned, box.main-content {{
                background-color: alpha(@window_bg_color, {alpha:.2});
            }}
            headerbar, headerbar.flat {{
                background-color: alpha(@window_bg_color, {header_alpha:.2});
            }}
            .sidebar {{
                background-color: alpha(@window_bg_color, {side_alpha:.2});
            }}
            window.dialog, window.dialog.background, dialog sheet {{
                background-color: alpha(@window_bg_color, {dialog_alpha:.2});
            }}
        ")
    }
}

pub fn get_theme_provider() -> gtk4::CssProvider {
    THEME_PROVIDER.with(|cell| {
        let mut opt = cell.borrow_mut();
        if let Some(ref provider) = *opt {
            provider.clone()
        } else {
            let provider = gtk4::CssProvider::new();
            if let Some(display) = gtk4::gdk::Display::default() {
                gtk4::style_context_add_provider_for_display(
                    &display,
                    &provider,
                    gtk4::STYLE_PROVIDER_PRIORITY_USER,
                );
            }
            *opt = Some(provider.clone());
            provider
        }
    })
}

pub fn apply_theme_css(roundness_percent: u32, opacity_percent: u32) {
    let (theme_colors, is_dark) = get_system_theme_css_and_dark();
    let base_css = include_str!("style.css");
    let radius_css = generate_radius_css(roundness_percent);
    let opacity_css = generate_opacity_css(opacity_percent);
    let full_css = format!("{theme_colors}\n{base_css}\n{radius_css}\n{opacity_css}");

    let provider = get_theme_provider();
    provider.load_from_data(&full_css);

    let style_manager = adw::StyleManager::default();
    if is_dark {
        style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
    } else {
        style_manager.set_color_scheme(adw::ColorScheme::ForceLight);
    }
}

pub fn reload_theme() {
    let settings = crate::ui::settings::AppSettings::load();
    apply_theme_css(settings.corner_roundness, settings.window_opacity);
}

pub fn setup_theme_watcher() {
    if WATCHER_INITIALIZED.swap(true, Ordering::SeqCst) {
        return;
    }

    LAST_THEME_ID.with(|cell| {
        *cell.borrow_mut() = get_current_theme_id();
    });

    // 1. Monitorear directorio de quickshell ~/.config/quickshell
    if let Some(home) = dirs::home_dir() {
        let quickshell_dir = home.join(".config/quickshell");
        if quickshell_dir.exists() {
            let gfile = gio::File::for_path(&quickshell_dir);
            if let Ok(mon) = gfile.monitor_directory(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE) {
                mon.connect_changed(move |_, _, _, _| {
                    let cur = get_current_theme_id();
                    let changed = LAST_THEME_ID.with(|cell| {
                        let mut last = cell.borrow_mut();
                        if *last != cur {
                            *last = cur;
                            true
                        } else {
                            false
                        }
                    });
                    if changed {
                        reload_theme();
                    }
                });
                FILE_MONITORS.with(|cell| cell.borrow_mut().push(mon));
            }
        }

        // 2. Monitorear ~/.config/gtk-4.0/gtk.css
        let gtk4_css = home.join(".config/gtk-4.0/gtk.css");
        if gtk4_css.exists() {
            let gfile = gio::File::for_path(&gtk4_css);
            if let Ok(mon) = gfile.monitor_file(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE) {
                mon.connect_changed(move |_, _, _, _| {
                    reload_theme();
                });
                FILE_MONITORS.with(|cell| cell.borrow_mut().push(mon));
            }
        }
    }

    // 3. Monitorear archivo de señal $XDG_RUNTIME_DIR/quickshell_theme_reload.toggle
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        let toggle = PathBuf::from(runtime_dir).join("quickshell_theme_reload.toggle");
        let gfile = gio::File::for_path(&toggle);
        if let Ok(mon) = gfile.monitor_file(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE) {
            mon.connect_changed(move |_, _, _, _| {
                let cur = get_current_theme_id();
                let changed = LAST_THEME_ID.with(|cell| {
                    let mut last = cell.borrow_mut();
                    if *last != cur {
                        *last = cur;
                        true
                    } else {
                        false
                    }
                });
                if changed {
                    reload_theme();
                }
            });
            FILE_MONITORS.with(|cell| cell.borrow_mut().push(mon));
        }
    }

    // 4. Chequeo periódico de seguridad cada 1500 ms (garantiza sincronización sin fallas por inotify swaps)
    glib::timeout_add_local(std::time::Duration::from_millis(1500), move || {
        let cur = get_current_theme_id();
        let changed = LAST_THEME_ID.with(|cell| {
            let mut last = cell.borrow_mut();
            if *last != cur {
                *last = cur;
                true
            } else {
                false
            }
        });
        if changed {
            reload_theme();
        }
        glib::ControlFlow::Continue
    });
}

pub fn get_available_icon_themes() -> Vec<String> {
    let mut dirs_to_check = vec![std::path::PathBuf::from("/usr/share/icons")];
    if let Some(home) = dirs::home_dir() {
        dirs_to_check.push(home.join(".local/share/icons"));
        dirs_to_check.push(home.join(".icons"));
    }

    let mut themes = std::collections::BTreeSet::new();
    for dir in dirs_to_check {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let index_theme = path.join("index.theme");
                    if index_theme.is_file() {
                        if let Ok(content) = std::fs::read_to_string(&index_theme) {
                            if content.contains("[Icon Theme]")
                                && (content.contains("Directories=") || content.contains("Inherits="))
                            {
                                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                    if name != "default" && name != "hicolor" && !name.starts_with('.') {
                                        themes.insert(name.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    themes.into_iter().collect()
}

pub fn apply_icon_theme(theme_name: &str) {
    if let Some(settings) = gtk4::Settings::default() {
        if theme_name.is_empty() || theme_name == "Predeterminado del sistema" {
            settings.reset_property("gtk-icon-theme-name");
        } else {
            settings.set_gtk_icon_theme_name(Some(theme_name));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_themes_validity() {
        for id in ["default", "street", "mountains", "noir", "anime-sunset"] {
            let th = get_builtin_theme(id).expect("Builtin theme should exist");
            assert_eq!(th.id, id);
            assert!(!th.name.is_empty());
            assert!(th.bg.starts_with('#'));
            assert!(th.primary.starts_with('#'));
            let css = theme_to_gtk_css(&th);
            assert!(css.contains("@define-color window_bg_color"));
            assert!(css.contains("@define-color accent_color"));
        }
    }

    #[test]
    fn test_load_current_theme() {
        let cur = load_current_mrdemonc_theme();
        assert!(cur.is_some(), "Current theme should load successfully");
        let th = cur.unwrap();
        assert!(!th.bg.is_empty());
        assert!(!th.primary.is_empty());
    }

    #[test]
    fn test_available_themes_list() {
        let themes = get_available_mrdemonc_themes();
        assert!(!themes.is_empty(), "Should list available themes");
        let ids: Vec<&str> = themes.iter().map(|(id, _)| id.as_str()).collect();
        assert!(ids.contains(&"anime-sunset"));
        assert!(ids.contains(&"noir"));
        assert!(ids.contains(&"default"));
    }

    #[test]
    fn test_system_theme_css_generation() {
        let (css, is_dark) = get_system_theme_css_and_dark();
        assert!(!css.is_empty());
        assert!(css.contains("@define-color window_bg_color"));
        assert!(css.contains("@define-color accent_color"));
        assert!(css.contains("@define-color card_bg_color"));
        assert!(is_dark);
    }

    #[test]
    fn test_hex_luminance() {
        let lum_white = hex_luminance("#ffffff");
        let lum_black = hex_luminance("#000000");
        assert!(lum_white > 0.9);
        assert!(lum_black < 0.05);
    }
}

