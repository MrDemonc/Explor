use gtk4::gdk::{Key, ModifierType};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActionId {
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    Copy,
    Cut,
    Paste,
    Extract,
    Compress,
    Trash,
    Rename,
    NewFile,
    NewFolder,
    Terminal,
    CopyPath,
    EditPath,
    ToggleViewMode,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    TogglePreview,
    ToggleSidebar,
    ToggleHidden,
    ToggleNavbar,
    Search,
    CommandPalette,
    OpenSettings,
    SelectNext,
    SelectPrev,
    OpenSelected,
    GoParent,
    GoFirst,
    GoLast,
    HistoryBack,
    HistoryForward,
    FocusSidebar,
    FocusList,
    ShowProgress,
}

#[derive(Clone, Debug)]
pub struct ActionMeta {
    pub id: ActionId,
    pub key_id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub default_shortcut: &'static str,
}

pub const ALL_ACTIONS: &[ActionMeta] = &[
    // Pestañas
    ActionMeta { id: ActionId::NewTab, key_id: "new_tab", name: "Open in new tab", category: "Tabs", default_shortcut: "Ctrl+T" },
    ActionMeta { id: ActionId::CloseTab, key_id: "close_tab", name: "Close current tab", category: "Tabs", default_shortcut: "Ctrl+W" },
    ActionMeta { id: ActionId::NextTab, key_id: "next_tab", name: "Next tab", category: "Tabs", default_shortcut: "Ctrl+Tab" },
    ActionMeta { id: ActionId::PrevTab, key_id: "prev_tab", name: "Previous tab", category: "Tabs", default_shortcut: "Ctrl+Shift+Tab" },

    // Gestión de Archivos
    ActionMeta { id: ActionId::Copy, key_id: "copy", name: "Copy file", category: "File Management", default_shortcut: "y" },
    ActionMeta { id: ActionId::Cut, key_id: "cut", name: "Cut file", category: "File Management", default_shortcut: "x" },
    ActionMeta { id: ActionId::Paste, key_id: "paste", name: "Paste file", category: "File Management", default_shortcut: "p" },
    ActionMeta { id: ActionId::Extract, key_id: "extract", name: "Extract archive", category: "File Management", default_shortcut: "e" },
    ActionMeta { id: ActionId::Compress, key_id: "compress", name: "Compress to ZIP", category: "File Management", default_shortcut: "z" },
    ActionMeta { id: ActionId::Trash, key_id: "trash", name: "Move to trash", category: "File Management", default_shortcut: "d" },
    ActionMeta { id: ActionId::Rename, key_id: "rename", name: "Rename file", category: "File Management", default_shortcut: "r" },
    ActionMeta { id: ActionId::NewFile, key_id: "new_file", name: "Create new file", category: "File Management", default_shortcut: "a" },
    ActionMeta { id: ActionId::NewFolder, key_id: "new_folder", name: "Create new folder", category: "File Management", default_shortcut: "Shift+A" },
    ActionMeta { id: ActionId::Terminal, key_id: "terminal", name: "Open terminal", category: "File Management", default_shortcut: "o" },
    ActionMeta { id: ActionId::CopyPath, key_id: "copy_path", name: "Copy current path", category: "File Management", default_shortcut: "Ctrl+Shift+C" },

    // Navegación
    ActionMeta { id: ActionId::OpenSelected, key_id: "open_selected", name: "Open folder or file", category: "Navigation", default_shortcut: "Enter" },
    ActionMeta { id: ActionId::GoParent, key_id: "go_parent", name: "Go to parent folder", category: "Navigation", default_shortcut: "Backspace" },
    ActionMeta { id: ActionId::SelectNext, key_id: "select_next", name: "Select next item", category: "Navigation", default_shortcut: "j" },
    ActionMeta { id: ActionId::SelectPrev, key_id: "select_prev", name: "Select previous item", category: "Navigation", default_shortcut: "k" },
    ActionMeta { id: ActionId::GoFirst, key_id: "go_first", name: "Go to first item", category: "Navigation", default_shortcut: "g" },
    ActionMeta { id: ActionId::GoLast, key_id: "go_last", name: "Go to last item", category: "Navigation", default_shortcut: "G" },
    ActionMeta { id: ActionId::FocusSidebar, key_id: "focus_sidebar", name: "Focus sidebar", category: "Navigation", default_shortcut: "←" },
    ActionMeta { id: ActionId::FocusList, key_id: "focus_list", name: "Focus file list", category: "Navigation", default_shortcut: "→" },
    ActionMeta { id: ActionId::HistoryBack, key_id: "history_back", name: "History back", category: "Navigation", default_shortcut: "Alt+←" },
    ActionMeta { id: ActionId::HistoryForward, key_id: "history_forward", name: "History forward", category: "Navigation", default_shortcut: "Alt+→" },
    ActionMeta { id: ActionId::EditPath, key_id: "edit_path", name: "Edit path manually", category: "Navigation", default_shortcut: "Ctrl+L" },

    // Vistas y Paneles
    ActionMeta { id: ActionId::ToggleViewMode, key_id: "toggle_view_mode", name: "Toggle list / grid view", category: "Views & Panels", default_shortcut: "v" },
    ActionMeta { id: ActionId::ZoomIn, key_id: "zoom_in", name: "Zoom in", category: "Views & Panels", default_shortcut: "Ctrl++" },
    ActionMeta { id: ActionId::ZoomOut, key_id: "zoom_out", name: "Zoom out", category: "Views & Panels", default_shortcut: "Ctrl+-" },
    ActionMeta { id: ActionId::ZoomReset, key_id: "zoom_reset", name: "Reset zoom (100%)", category: "Views & Panels", default_shortcut: "Ctrl+0" },
    ActionMeta { id: ActionId::TogglePreview, key_id: "toggle_preview", name: "Toggle preview", category: "Views & Panels", default_shortcut: "Space" },
    ActionMeta { id: ActionId::ToggleSidebar, key_id: "toggle_sidebar", name: "Toggle sidebar (collapse/expand)", category: "Views & Panels", default_shortcut: "Ctrl+B" },
    ActionMeta { id: ActionId::ToggleHidden, key_id: "toggle_hidden", name: "Toggle hidden files", category: "Views & Panels", default_shortcut: "Ctrl+H" },
    ActionMeta { id: ActionId::ToggleNavbar, key_id: "toggle_navbar", name: "Toggle top navigation bar", category: "Views & Panels", default_shortcut: "F10" },
    ActionMeta { id: ActionId::Search, key_id: "search", name: "Filter in real time", category: "Views & Panels", default_shortcut: "Ctrl+F" },
    ActionMeta { id: ActionId::CommandPalette, key_id: "command_palette", name: "Open command palette", category: "Views & Panels", default_shortcut: "Ctrl+P" },
    ActionMeta { id: ActionId::OpenSettings, key_id: "open_settings", name: "Open Settings", category: "Views & Panels", default_shortcut: "Ctrl+," },
    ActionMeta { id: ActionId::ShowProgress, key_id: "show_progress", name: "Show operations progress", category: "Views & Panels", default_shortcut: "b" },
];

#[derive(Clone, Debug)]
pub struct ShortcutsManager {
    custom: HashMap<String, String>,
}

impl ShortcutsManager {
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("explor").join("shortcuts.json"))
    }

    pub fn load() -> Self {
        let mut custom = HashMap::new();
        if let Some(path) = Self::config_path() {
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if let Some((k, v)) = trimmed.split_once(':') {
                            let key_id = k.trim().trim_matches('"');
                            let shortcut_val = v.trim().trim_matches(',').trim().trim_matches('"');
                            if !key_id.is_empty() && !shortcut_val.is_empty() {
                                custom.insert(key_id.to_string(), shortcut_val.to_string());
                            }
                        }
                    }
                }
            }
        }
        Self { custom }
    }

    pub fn save(&self) {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let mut entries = Vec::new();
            for (k, v) in &self.custom {
                entries.push(format!("  \"{}\": \"{}\"", k, v));
            }
            let json = format!("{{\n{}\n}}\n", entries.join(",\n"));
            let _ = std::fs::write(path, json);
        }
    }

    pub fn get_shortcut(&self, action: ActionId) -> String {
        let meta = ALL_ACTIONS.iter().find(|a| a.id == action).unwrap();
        if let Some(val) = self.custom.get(meta.key_id) {
            val.clone()
        } else {
            meta.default_shortcut.to_string()
        }
    }

    pub fn is_custom(&self, action: ActionId) -> bool {
        let meta = ALL_ACTIONS.iter().find(|a| a.id == action).unwrap();
        self.custom.contains_key(meta.key_id)
    }

    pub fn reset_shortcut(&mut self, action: ActionId) {
        let meta = ALL_ACTIONS.iter().find(|a| a.id == action).unwrap();
        self.custom.remove(meta.key_id);
        self.save();
    }

    pub fn reset_defaults(&mut self) {
        self.custom.clear();
        self.save();
    }

    /// Asigna un nuevo atajo comprobando que NO esté ya en uso por otra acción (detección y bloqueo de colisiones)
    pub fn set_shortcut(&mut self, target_action: ActionId, new_shortcut: String) -> Result<(), String> {
        let target_meta = ALL_ACTIONS.iter().find(|a| a.id == target_action).unwrap();

        // 1. Comprobar si otra acción ya tiene asignado exactamente este atajo
        for action in ALL_ACTIONS {
            if action.id != target_action {
                let active_sc = self.get_shortcut(action.id);
                if active_sc.eq_ignore_ascii_case(&new_shortcut) {
                    return Err(format!(
                        "Shortcut '{}' is already in use by '{}'. Duplicate commands are not allowed.",
                        new_shortcut, action.name
                    ));
                }
            }
        }

        // 2. Si no hay colisión, guardar
        self.custom.insert(target_meta.key_id.to_string(), new_shortcut);
        self.save();
        Ok(())
    }

    /// Convierte un evento de tecla de GTK4 en una representación canónica de atajo
    pub fn event_to_shortcut(key: Key, modifier: ModifierType) -> Option<String> {
        // Ignorar pulsaciones de teclas modificadoras solas
        match key {
            Key::Control_L | Key::Control_R |
            Key::Shift_L | Key::Shift_R |
            Key::Alt_L | Key::Alt_R |
            Key::Super_L | Key::Super_R |
            Key::Meta_L | Key::Meta_R |
            Key::Caps_Lock | Key::Num_Lock => return None,
            _ => {}
        }

        let ctrl = modifier.contains(ModifierType::CONTROL_MASK);
        let alt = modifier.contains(ModifierType::ALT_MASK);
        let shift = modifier.contains(ModifierType::SHIFT_MASK);

        let key_str = match key {
            Key::Return | Key::KP_Enter => "Enter".to_string(),
            Key::BackSpace => "Backspace".to_string(),
            Key::space => "Space".to_string(),
            Key::Tab | Key::KP_Tab | Key::ISO_Left_Tab => "Tab".to_string(),
            Key::Escape => "Esc".to_string(),
            Key::Left => "←".to_string(),
            Key::Right => "→".to_string(),
            Key::Up => "↑".to_string(),
            Key::Down => "↓".to_string(),
            Key::comma => ",".to_string(),
            Key::period => ".".to_string(),
            Key::slash => "/".to_string(),
            Key::plus | Key::equal | Key::KP_Add => "+".to_string(),
            Key::minus | Key::KP_Subtract => "-".to_string(),
            Key::_0 | Key::KP_0 => "0".to_string(),
            Key::_1 | Key::KP_1 => "1".to_string(),
            Key::_2 | Key::KP_2 => "2".to_string(),
            Key::_3 | Key::KP_3 => "3".to_string(),
            Key::_4 | Key::KP_4 => "4".to_string(),
            Key::_5 | Key::KP_5 => "5".to_string(),
            Key::_6 | Key::KP_6 => "6".to_string(),
            Key::_7 | Key::KP_7 => "7".to_string(),
            Key::_8 | Key::KP_8 => "8".to_string(),
            Key::_9 | Key::KP_9 => "9".to_string(),
            Key::F1 => "F1".to_string(),
            Key::F2 => "F2".to_string(),
            Key::F3 => "F3".to_string(),
            Key::F4 => "F4".to_string(),
            Key::F5 => "F5".to_string(),
            Key::F6 => "F6".to_string(),
            Key::F7 => "F7".to_string(),
            Key::F8 => "F8".to_string(),
            Key::F9 => "F9".to_string(),
            Key::F10 => "F10".to_string(),
            Key::F11 => "F11".to_string(),
            Key::F12 => "F12".to_string(),
            _ => {
                if let Some(c) = key.to_unicode() {
                    c.to_string()
                } else if let Some(name) = key.name() {
                    name.to_string()
                } else {
                    return None;
                }
            }
        };

        if ctrl || alt || (shift && (key_str.len() > 1 || ctrl)) {
            let mut parts = Vec::new();
            if ctrl {
                parts.push("Ctrl");
            }
            if alt {
                parts.push("Alt");
            }
            if shift && (key_str.len() > 1 || ctrl) {
                parts.push("Shift");
            }
            let k = if key_str.len() == 1 && key_str.chars().next().unwrap().is_alphabetic() {
                key_str.to_uppercase()
            } else {
                key_str
            };
            parts.push(&k);
            Some(parts.join("+"))
        } else if shift && key_str.len() == 1 && key_str.chars().next().unwrap().is_alphabetic() {
            if key_str == "G" {
                Some("G".to_string())
            } else {
                Some(format!("Shift+{}", key_str.to_uppercase()))
            }
        } else {
            Some(key_str)
        }
    }

    /// Compara un evento de tecla contra todos los atajos activos y devuelve la acción coincidente
    pub fn match_event(&self, key: Key, modifier: ModifierType) -> Option<ActionId> {
        let event_shortcut = Self::event_to_shortcut(key, modifier)?;
        for action in ALL_ACTIONS {
            let cur = self.get_shortcut(action.id);
            if cur.eq_ignore_ascii_case(&event_shortcut) {
                return Some(action.id);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_shortcuts() {
        let mgr = ShortcutsManager { custom: HashMap::new() };
        assert_eq!(mgr.get_shortcut(ActionId::NewTab), "Ctrl+T");
        assert_eq!(mgr.get_shortcut(ActionId::Copy), "y");
        assert_eq!(mgr.get_shortcut(ActionId::ToggleViewMode), "v");
    }

    #[test]
    fn test_collision_detection() {
        let mut mgr = ShortcutsManager { custom: HashMap::new() };
        // Trying to set Copy ("y") to "Ctrl+T" which is used by NewTab
        let res = mgr.set_shortcut(ActionId::Copy, "Ctrl+T".to_string());
        assert!(res.is_err());
        let err_msg = res.err().unwrap();
        assert!(err_msg.contains("Ctrl+T"));
        assert!(err_msg.contains("Open in new tab"));

        // Copy must still be default "y"
        assert_eq!(mgr.get_shortcut(ActionId::Copy), "y");
    }

    #[test]
    fn test_set_and_reset_shortcut() {
        let mut mgr = ShortcutsManager { custom: HashMap::new() };
        // Assign unused shortcut to NewTab
        let res = mgr.set_shortcut(ActionId::NewTab, "Ctrl+Alt+N".to_string());
        assert!(res.is_ok());
        assert_eq!(mgr.get_shortcut(ActionId::NewTab), "Ctrl+Alt+N");
        assert!(mgr.is_custom(ActionId::NewTab));

        // Reset NewTab
        mgr.reset_shortcut(ActionId::NewTab);
        assert_eq!(mgr.get_shortcut(ActionId::NewTab), "Ctrl+T");
        assert!(!mgr.is_custom(ActionId::NewTab));
    }
}

