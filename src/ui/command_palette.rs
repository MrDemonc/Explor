use std::path::PathBuf;
use crate::ui::keybindings::{ActionId, ShortcutsManager};
use libadwaita as adw;
use libadwaita::prelude::*;
use gtk4::{Align, Box, Entry, EventControllerKey, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow};
use gtk4::gdk::Key;

#[derive(Clone)]
pub enum PaletteAction {
    Navigate(PathBuf),
    ToggleHidden,
    TogglePreview,
    ToggleSidebar,
    ToggleNavbar,
    OpenTerminal,
    NewFile,
    NewFolder,
    Copy,
    Cut,
    Paste,
    Extract,
    Compress,
    CopyPath,
    Delete,
    Rename,
    OpenSettings,
    SelectNext,
    SelectPrev,
    OpenSelected,
    GoParent,
    GoFirst,
    GoLast,
    GoBack,
    GoForward,
    EditPath,
    SearchFilter,
    ToggleViewMode,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    OpenShortcuts,
    ShowProgress,
    SetTheme(String),
}

#[derive(Clone)]
struct PaletteItem {
    title: String,
    subtitle: String,
    shortcut: Option<String>,
    icon: &'static str,
    action: PaletteAction,
}

pub fn show_command_palette<W: IsA<gtk4::Widget>, F>(
    parent: &W,
    current_path: PathBuf,
    shortcuts: &ShortcutsManager,
    on_action: F,
) where
    F: Fn(PaletteAction) + 'static + Clone,
{
    let dialog = adw::Dialog::builder()
        .title("Commands & Shortcuts")
        .content_width(580)
        .content_height(520)
        .build();

    let root_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .build();

    let header_bar = adw::HeaderBar::builder()
        .show_end_title_buttons(true)
        .build();
    root_box.append(&header_bar);

    let search_container = Box::builder()
        .orientation(Orientation::Horizontal)
        .margin_start(16)
        .margin_end(16)
        .margin_bottom(6)
        .build();

    let entry = Entry::builder()
        .placeholder_text("Search commands, shortcuts or paths (~/, /etc)...")
        .primary_icon_name("input-keyboard-symbolic")
        .hexpand(true)
        .css_classes(["command-palette-entry"])
        .build();
    search_container.append(&entry);

    let scrolled = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true)
        .margin_start(16)
        .margin_end(16)
        .margin_bottom(16)
        .build();

    let list_box = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();

    let mut items: Vec<PaletteItem> = Vec::new();

    // 1. Acciones principales
    items.push(PaletteItem {
        title: "Copy Selected File".to_string(),
        subtitle: "Copy selected item to clipboard".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::Copy)),
        icon: "edit-copy-symbolic",
        action: PaletteAction::Copy,
    });
    items.push(PaletteItem {
        title: "Cut Selected File".to_string(),
        subtitle: "Move selected item to clipboard".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::Cut)),
        icon: "edit-cut-symbolic",
        action: PaletteAction::Cut,
    });
    items.push(PaletteItem {
        title: "Paste File".to_string(),
        subtitle: "Paste item from clipboard into current folder".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::Paste)),
        icon: "edit-paste-symbolic",
        action: PaletteAction::Paste,
    });
    items.push(PaletteItem {
        title: "Extract Archive".to_string(),
        subtitle: "Extract .zip, .tar.*, .7z, .rar files into a folder".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::Extract)),
        icon: "package-x-generic",
        action: PaletteAction::Extract,
    });
    items.push(PaletteItem {
        title: "Compress to ZIP".to_string(),
        subtitle: "Compress selected file or folder into a .zip archive".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::Compress)),
        icon: "package-x-generic-symbolic",
        action: PaletteAction::Compress,
    });
    items.push(PaletteItem {
        title: "Copy Current Path".to_string(),
        subtitle: "Copy current folder path to clipboard".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::CopyPath)),
        icon: "edit-copy-symbolic",
        action: PaletteAction::CopyPath,
    });
    items.push(PaletteItem {
        title: "Move to Trash".to_string(),
        subtitle: "Safely delete selected item".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::Trash)),
        icon: "user-trash-symbolic",
        action: PaletteAction::Delete,
    });
    items.push(PaletteItem {
        title: "Rename File".to_string(),
        subtitle: "Rename selected file or folder".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::Rename)),
        icon: "document-edit-symbolic",
        action: PaletteAction::Rename,
    });
    items.push(PaletteItem {
        title: "Create New File".to_string(),
        subtitle: "Create an empty file in current folder".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::NewFile)),
        icon: "document-new-symbolic",
        action: PaletteAction::NewFile,
    });
    items.push(PaletteItem {
        title: "Create New Folder".to_string(),
        subtitle: "Create a directory in current folder".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::NewFolder)),
        icon: "folder-new-symbolic",
        action: PaletteAction::NewFolder,
    });
    items.push(PaletteItem {
        title: "Open Terminal Here".to_string(),
        subtitle: format!("Launch terminal in {}", current_path.display()),
        shortcut: Some(shortcuts.get_shortcut(ActionId::Terminal)),
        icon: "utilities-terminal-symbolic",
        action: PaletteAction::OpenTerminal,
    });

    // 2. Vistas y navegación
    items.push(PaletteItem {
        title: "Toggle Side Preview".to_string(),
        subtitle: "Show or hide file details and preview panel".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::TogglePreview)),
        icon: "sidebar-show-right-symbolic",
        action: PaletteAction::TogglePreview,
    });
    items.push(PaletteItem {
        title: "Toggle Sidebar (Collapse/Expand)".to_string(),
        subtitle: "Switch between normal and icons-only mode".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::ToggleSidebar)),
        icon: "sidebar-show-symbolic",
        action: PaletteAction::ToggleSidebar,
    });
    items.push(PaletteItem {
        title: "Toggle Hidden Files".to_string(),
        subtitle: "Show or hide dotfiles".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::ToggleHidden)),
        icon: "view-conceal-symbolic",
        action: PaletteAction::ToggleHidden,
    });
    items.push(PaletteItem {
        title: "Toggle Navigation Bar".to_string(),
        subtitle: "Hide or show the top bar".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::ToggleNavbar)),
        icon: "view-fullscreen-symbolic",
        action: PaletteAction::ToggleNavbar,
    });
    items.push(PaletteItem {
        title: "Filter in Real Time".to_string(),
        subtitle: "Activate instant search in current directory".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::Search)),
        icon: "system-search-symbolic",
        action: PaletteAction::SearchFilter,
    });
    items.push(PaletteItem {
        title: "Edit Path Manually".to_string(),
        subtitle: "Type destination path directly".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::EditPath)),
        icon: "document-edit-symbolic",
        action: PaletteAction::EditPath,
    });
    items.push(PaletteItem {
        title: "Open Folder or File".to_string(),
        subtitle: "Open directory or launch file".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::OpenSelected)),
        icon: "go-next-symbolic",
        action: PaletteAction::OpenSelected,
    });
    items.push(PaletteItem {
        title: "Go to Parent Folder".to_string(),
        subtitle: "Navigate to upper directory".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::GoParent)),
        icon: "go-up-symbolic",
        action: PaletteAction::GoParent,
    });
    items.push(PaletteItem {
        title: "Select Next Item".to_string(),
        subtitle: "Move selection cursor to next item".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::SelectNext)),
        icon: "go-down-symbolic",
        action: PaletteAction::SelectNext,
    });
    items.push(PaletteItem {
        title: "Select Previous Item".to_string(),
        subtitle: "Move selection cursor to previous item".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::SelectPrev)),
        icon: "go-up-symbolic",
        action: PaletteAction::SelectPrev,
    });
    items.push(PaletteItem {
        title: "Go to First Item".to_string(),
        subtitle: "Select the first item".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::GoFirst)),
        icon: "go-top-symbolic",
        action: PaletteAction::GoFirst,
    });
    items.push(PaletteItem {
        title: "Go to Last Item".to_string(),
        subtitle: "Select the last item".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::GoLast)),
        icon: "go-bottom-symbolic",
        action: PaletteAction::GoLast,
    });
    items.push(PaletteItem {
        title: "History Back".to_string(),
        subtitle: "Go back to previous location".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::HistoryBack)),
        icon: "go-previous-symbolic",
        action: PaletteAction::GoBack,
    });
    items.push(PaletteItem {
        title: "History Forward".to_string(),
        subtitle: "Go forward in location history".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::HistoryForward)),
        icon: "go-next-symbolic",
        action: PaletteAction::GoForward,
    });
    items.push(PaletteItem {
        title: "Open Settings / Preferences".to_string(),
        subtitle: "Configure behavior, bars and theme".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::OpenSettings)),
        icon: "emblem-system-symbolic",
        action: PaletteAction::OpenSettings,
    });
    items.push(PaletteItem {
        title: "Toggle List / Grid View".to_string(),
        subtitle: "Switch between list rows and card grid".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::ToggleViewMode)),
        icon: "view-grid-symbolic",
        action: PaletteAction::ToggleViewMode,
    });
    items.push(PaletteItem {
        title: "Zoom In".to_string(),
        subtitle: "Increase size of icons and view elements".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::ZoomIn)),
        icon: "zoom-in-symbolic",
        action: PaletteAction::ZoomIn,
    });
    items.push(PaletteItem {
        title: "Zoom Out".to_string(),
        subtitle: "Decrease size of icons and view elements".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::ZoomOut)),
        icon: "zoom-out-symbolic",
        action: PaletteAction::ZoomOut,
    });
    items.push(PaletteItem {
        title: "Reset Zoom (100%)".to_string(),
        subtitle: "Restore default 100% zoom".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::ZoomReset)),
        icon: "zoom-original-symbolic",
        action: PaletteAction::ZoomReset,
    });
    items.push(PaletteItem {
        title: "Open in New Tab".to_string(),
        subtitle: "Open folder or bookmark in a new tab".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::NewTab)),
        icon: "tab-new-symbolic",
        action: PaletteAction::NewTab,
    });
    items.push(PaletteItem {
        title: "Close Current Tab".to_string(),
        subtitle: "Close active tab".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::CloseTab)),
        icon: "window-close-symbolic",
        action: PaletteAction::CloseTab,
    });
    items.push(PaletteItem {
        title: "Next Tab".to_string(),
        subtitle: "Switch to next tab".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::NextTab)),
        icon: "go-next-symbolic",
        action: PaletteAction::NextTab,
    });
    items.push(PaletteItem {
        title: "Previous Tab".to_string(),
        subtitle: "Switch to previous tab".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::PrevTab)),
        icon: "go-previous-symbolic",
        action: PaletteAction::PrevTab,
    });
    items.push(PaletteItem {
        title: "Customize Keyboard Shortcuts".to_string(),
        subtitle: "Edit commands with collision prevention and defaults restore".to_string(),
        shortcut: None,
        icon: "input-keyboard-symbolic",
        action: PaletteAction::OpenShortcuts,
    });
    items.push(PaletteItem {
        title: "Show Operations Progress".to_string(),
        subtitle: "Display status of active tasks in sidebar".to_string(),
        shortcut: Some(shortcuts.get_shortcut(ActionId::ShowProgress)),
        icon: "document-save-symbolic",
        action: PaletteAction::ShowProgress,
    });

    // 3. Rutas comunes
    if let Some(home) = dirs::home_dir() {
        items.push(PaletteItem {
            title: "Go to Home".to_string(),
            subtitle: home.display().to_string(),
            shortcut: Some("~".to_string()),
            icon: "user-home-symbolic",
            action: PaletteAction::Navigate(home.clone()),
        });
        items.push(PaletteItem {
            title: "Go to Projects".to_string(),
            subtitle: home.join("Documents/Proyects").display().to_string(),
            shortcut: None,
            icon: "folder-symbolic",
            action: PaletteAction::Navigate(home.join("Documents/Proyects")),
        });
    }
    if let Some(p) = dirs::download_dir() {
        items.push(PaletteItem {
            title: "Go to Downloads".to_string(),
            subtitle: p.display().to_string(),
            shortcut: None,
            icon: "folder-download-symbolic",
            action: PaletteAction::Navigate(p),
        });
    }
    if let Some(p) = dirs::document_dir() {
        items.push(PaletteItem {
            title: "Go to Documents".to_string(),
            subtitle: p.display().to_string(),
            shortcut: None,
            icon: "folder-documents-symbolic",
            action: PaletteAction::Navigate(p),
        });
    }
    items.push(PaletteItem {
        title: "Go to System Root (/)".to_string(),
        subtitle: "/".to_string(),
        shortcut: Some("/".to_string()),
        icon: "drive-harddisk-symbolic",
        action: PaletteAction::Navigate(PathBuf::from("/")),
    });

    // 4. Temas de MrDemonc-SHELL
    let cur_theme_id = crate::ui::theme::get_current_theme_id().unwrap_or_default();
    let shell_themes = crate::ui::theme::get_available_mrdemonc_themes();
    for (t_id, t_name) in shell_themes {
        let is_current = t_id == cur_theme_id;
        let subtitle = if is_current {
            "Active MrDemonc-SHELL theme".to_string()
        } else {
            "Switch MrDemonc-SHELL theme".to_string()
        };
        items.push(PaletteItem {
            title: format!("Theme: {t_name}"),
            subtitle,
            shortcut: None,
            icon: "preferences-desktop-theme-symbolic",
            action: PaletteAction::SetTheme(t_id),
        });
    }

    // Función para repoblar la lista según el filtro
    let list_box_clone = list_box.clone();
    let items_ref = items.clone();

    let populate_list = move |filter: &str| {
        while let Some(child) = list_box_clone.first_child() {
            list_box_clone.remove(&child);
        }

        let filter_lower = filter.trim().to_lowercase();

        for (idx, item) in items_ref.iter().enumerate() {
            let matches = filter_lower.is_empty()
                || item.title.to_lowercase().contains(&filter_lower)
                || item.subtitle.to_lowercase().contains(&filter_lower)
                || item.shortcut.as_ref().map(|s| s.to_lowercase().contains(&filter_lower)).unwrap_or(false);

            if matches {
                let row = ListBoxRow::new();
                row.set_widget_name(&format!("{idx}"));

                let row_box = Box::builder()
                    .orientation(Orientation::Horizontal)
                    .spacing(10)
                    .margin_top(6)
                    .margin_bottom(6)
                    .margin_start(10)
                    .margin_end(10)
                    .build();

                let icon = gtk4::Image::builder()
                    .icon_name(item.icon)
                    .pixel_size(18)
                    .build();

                let text_box = Box::builder()
                    .orientation(Orientation::Vertical)
                    .spacing(2)
                    .hexpand(true)
                    .build();

                let title_label = Label::builder()
                    .label(&item.title)
                    .halign(Align::Start)
                    .css_classes(["heading"])
                    .build();

                let sub_label = Label::builder()
                    .label(&item.subtitle)
                    .halign(Align::Start)
                    .css_classes(["caption", "dim-label"])
                    .build();

                text_box.append(&title_label);
                text_box.append(&sub_label);

                row_box.append(&icon);
                row_box.append(&text_box);

                if let Some(ref sc) = item.shortcut {
                    let badge = Label::builder()
                        .label(sc)
                        .css_classes(["caption", "dim-label", "bubble-shortcut-badge", "monospace"])
                        .valign(Align::Center)
                        .build();
                    row_box.append(&badge);
                }

                row.set_child(Some(&row_box));
                list_box_clone.append(&row);
            }
        }

        if let Some(first) = list_box_clone.row_at_index(0) {
            list_box_clone.select_row(Some(&first));
        }
    };

    populate_list("");

    // Manejo de filtrado al escribir
    let pop_clone = populate_list.clone();
    entry.connect_changed(move |e| {
        pop_clone(e.text().as_str());
    });

    // Manejo de activación
    let on_act = on_action.clone();
    let dlg_weak = dialog.downgrade();
    let items_clone = items.clone();
    let current_path_clone = current_path.clone();

    let execute_selection = move |entry_text: &str, list_box: &ListBox| {
        let text = entry_text.trim();

        // 1. Si parece una ruta explícita (~/, /..., ../)
        let resolved_path = if text.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                let rest = text.trim_start_matches('~').trim_start_matches('/');
                Some(home.join(rest))
            } else {
                None
            }
        } else if text.starts_with('/') {
            Some(PathBuf::from(text))
        } else if text.starts_with('.') {
            Some(current_path_clone.join(text))
        } else {
            None
        };

        if let Some(path) = resolved_path {
            if path.exists() {
                on_act(PaletteAction::Navigate(path));
                if let Some(d) = dlg_weak.upgrade() {
                    d.close();
                }
                return;
            }
        }

        // 2. Si hay un elemento seleccionado en la lista
        if let Some(selected_row) = list_box.selected_row() {
            if let Ok(idx) = selected_row.widget_name().parse::<usize>() {
                if let Some(item) = items_clone.get(idx) {
                    on_act(item.action.clone());
                    if let Some(d) = dlg_weak.upgrade() {
                        d.close();
                    }
                }
            }
        }
    };

    let exec_clone = execute_selection.clone();
    let list_clone = list_box.clone();
    entry.connect_activate(move |e| {
        exec_clone(e.text().as_str(), &list_clone);
    });

    let exec_clone2 = execute_selection;
    let entry_clone = entry.clone();
    list_box.connect_row_activated(move |lb, _| {
        exec_clone2(entry_clone.text().as_str(), lb);
    });

    // Controlador de teclas para la paleta (Escape, Arriba, Abajo)
    let key_controller = EventControllerKey::new();
    let list_key = list_box.clone();
    let dlg_weak_key = dialog.downgrade();

    key_controller.connect_key_pressed(move |_, key, _, _| {
        match key {
            Key::Escape => {
                if let Some(d) = dlg_weak_key.upgrade() {
                    d.close();
                }
                glib::Propagation::Stop
            }
            Key::Down => {
                if let Some(cur) = list_key.selected_row() {
                    let next_idx = cur.index() + 1;
                    if let Some(next_row) = list_key.row_at_index(next_idx) {
                        list_key.select_row(Some(&next_row));
                        next_row.grab_focus();
                    }
                } else if let Some(first) = list_key.row_at_index(0) {
                    list_key.select_row(Some(&first));
                    first.grab_focus();
                }
                glib::Propagation::Stop
            }
            Key::Up => {
                if let Some(cur) = list_key.selected_row() {
                    let prev_idx = cur.index() - 1;
                    if prev_idx >= 0 {
                        if let Some(prev_row) = list_key.row_at_index(prev_idx) {
                            list_key.select_row(Some(&prev_row));
                            prev_row.grab_focus();
                        }
                    }
                }
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        }
    });

    dialog.add_controller(key_controller);

    scrolled.set_child(Some(&list_box));
    root_box.append(&search_container);
    root_box.append(&scrolled);
    dialog.set_child(Some(&root_box));

    dialog.present(Some(parent));
    entry.grab_focus();
}
