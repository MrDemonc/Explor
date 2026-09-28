use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use gtk4::prelude::*;
use gtk4::{
    Align, Box, EventControllerKey, Label, ListBox, ListBoxRow, Orientation, ProgressBar, ScrolledWindow, SelectionMode,
};
use gtk4::gdk::{self, Key};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;
use gtk4::{Button, Image, Popover};
use crate::ui::progress_dialog::{send_system_notification, ProgressMessage};
use crate::fs::devices::{get_storage_devices, DeviceInfo};

#[derive(Clone)]
pub struct Bookmark {
    pub name: String,
    pub icon: String,
    pub path: PathBuf,
    pub shortcut_hint: Option<String>,
    pub is_removable: bool,
}

#[derive(Clone)]
pub struct Sidebar {
    container: ScrolledWindow,
    bookmarks_list: ListBox,
    devices_list: ListBox,
    bookmarks: Rc<RefCell<Vec<Bookmark>>>,
    devices: Rc<RefCell<Vec<DeviceInfo>>>,
    is_collapsed: Rc<RefCell<bool>>,
    reorder_source_idx: Rc<RefCell<Option<usize>>>,
    
    // Elementos a ocultar/mostrar en modo colapsado
    bookmark_labels: Rc<RefCell<Vec<Label>>>,
    bookmark_hints: Rc<RefCell<Vec<Label>>>,
    bookmark_boxes: Rc<RefCell<Vec<Box>>>,
    bookmarks_header: Label,
    
    device_widgets: Rc<RefCell<Vec<(Box, Label, Option<Label>, Option<ProgressBar>)>>>,
    devices_header: Label,
    
    on_navigate_cb: Rc<RefCell<Option<Rc<dyn Fn(PathBuf)>>>>,
    on_empty_trash_cb: Rc<RefCell<Option<Rc<dyn Fn()>>>>,

    // Menú contextual para marcadores
    bookmark_popover: Popover,
    bookmark_menu_box: Box,

    // Progreso de múltiples operaciones concurrentes
    op_header: Label,
    op_cards_container: Box,
    collapsed_btn: Button,
    popover: Popover,
    pop_cards_container: Box,
    active_tasks_count: Rc<RefCell<usize>>,
}

impl Sidebar {
    pub fn new() -> Self {
        let is_collapsed = Rc::new(RefCell::new(false));
        let on_navigate_cb: Rc<RefCell<Option<Rc<dyn Fn(PathBuf)>>>> = Rc::new(RefCell::new(None));
        let on_empty_trash_cb: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));

        let container = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .width_request(130)
            .css_classes(["sidebar", "view"])
            .build();

        let root_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .margin_top(4)
            .margin_bottom(4)
            .margin_start(3)
            .margin_end(3)
            .vexpand(true)
            .build();

        // 1. MARCADORES
        let bookmarks_header = Label::builder()
            .label("BOOKMARKS")
            .halign(Align::Start)
            .margin_start(6)
            .margin_top(2)
            .margin_bottom(2)
            .css_classes(["heading", "caption", "dim-label"])
            .build();

        let bookmarks_list = ListBox::builder()
            .selection_mode(SelectionMode::None)
            .css_classes(["navigation-sidebar", "rich-list"])
            .build();

        let initial_bookmarks = Self::load_bookmarks();
        let bookmarks = Rc::new(RefCell::new(initial_bookmarks));
        let bookmark_labels = Rc::new(RefCell::new(Vec::new()));
        let bookmark_hints = Rc::new(RefCell::new(Vec::new()));
        let bookmark_boxes = Rc::new(RefCell::new(Vec::new()));
        let reorder_source_idx = Rc::new(RefCell::new(None));

        // 2. DISPOSITIVOS
        let devices_header = Label::builder()
            .label("DEVICES")
            .halign(Align::Start)
            .margin_start(6)
            .margin_top(6)
            .margin_bottom(2)
            .css_classes(["heading", "caption", "dim-label"])
            .build();

        let devices_list = ListBox::builder()
            .selection_mode(SelectionMode::None)
            .css_classes(["navigation-sidebar", "rich-list"])
            .build();

        let devices = Rc::new(RefCell::new(get_storage_devices()));
        let device_widgets = Rc::new(RefCell::new(Vec::new()));

        // SECCIÓN DE OPERACIONES ACTIVAS MULTI-PROCESO
        let op_header = Label::builder()
            .label("ACTIVE OPERATIONS")
            .halign(Align::Start)
            .margin_start(8)
            .margin_top(12)
            .css_classes(["heading", "caption", "dim-label"])
            .visible(false)
            .build();

        let op_cards_container = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .margin_bottom(6)
            .visible(false)
            .build();

        let collapsed_icon = Image::builder()
            .icon_name("document-save-symbolic")
            .pixel_size(18)
            .build();

        let collapsed_btn = Button::builder()
            .child(&collapsed_icon)
            .css_classes(["flat", "circular", "collapsed-op-btn"])
            .halign(Align::Center)
            .tooltip_text("Operations in progress (Press 'b' or hover)")
            .visible(false)
            .build();

        let popover = Popover::builder()
            .position(gtk4::PositionType::Right)
            .autohide(true)
            .build();
        popover.set_parent(&collapsed_btn);

        let pop_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .max_content_height(360)
            .propagate_natural_height(true)
            .build();

        let pop_cards_container = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(6)
            .margin_start(10)
            .margin_end(10)
            .margin_top(8)
            .margin_bottom(8)
            .width_request(240)
            .build();

        pop_scroll.set_child(Some(&pop_cards_container));
        popover.set_child(Some(&pop_scroll));

        let pop_motion = popover.clone();
        let motion = gtk4::EventControllerMotion::new();
        motion.connect_enter(move |_, _, _| {
            pop_motion.popup();
        });
        collapsed_btn.add_controller(motion);

        let pop_click = popover.clone();
        collapsed_btn.connect_clicked(move |_| {
            if pop_click.is_visible() {
                pop_click.popdown();
            } else {
                pop_click.popup();
            }
        });

        let active_tasks_count = Rc::new(RefCell::new(0));

        root_box.append(&bookmarks_header);
        root_box.append(&bookmarks_list);
        root_box.append(&devices_header);
        root_box.append(&devices_list);
        root_box.append(&op_header);
        root_box.append(&op_cards_container);
        root_box.append(&collapsed_btn);

        container.set_child(Some(&root_box));

        let bookmark_menu_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(1)
            .css_classes(["context-menu-box", "sidebar-context-menu"])
            .build();

        let bookmark_popover = Popover::builder()
            .autohide(true)
            .has_arrow(false)
            .child(&bookmark_menu_box)
            .css_classes(["menu", "context-menu-popover"])
            .build();

        bookmark_popover.set_parent(&bookmarks_list);

        let sidebar = Self {
            container,
            bookmarks_list,
            devices_list,
            bookmarks,
            devices,
            is_collapsed,
            reorder_source_idx,
            bookmark_labels,
            bookmark_hints,
            bookmark_boxes,
            bookmarks_header,
            device_widgets,
            devices_header,
            on_navigate_cb,
            on_empty_trash_cb,
            bookmark_popover,
            bookmark_menu_box,
            op_header,
            op_cards_container,
            collapsed_btn,
            popover,
            pop_cards_container,
            active_tasks_count,
        };

        sidebar.build_bookmark_rows();
        sidebar.build_device_rows();
        sidebar.setup_volume_monitor();
        sidebar.setup_signals();
        sidebar.setup_drag_and_drop();

        sidebar
    }

    fn bookmarks_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("explor").join("bookmarks.txt"))
    }

    fn load_bookmarks() -> Vec<Bookmark> {
        let mut loaded = Vec::new();
        if let Some(path) = Self::bookmarks_config_path() {
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    for line in content.lines() {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }
                        if line.contains('|') {
                            let parts: Vec<&str> = line.split('|').collect();
                            if parts.len() >= 4 {
                                let name = parts[0].to_string();
                                let icon = parts[1].to_string();
                                let p = PathBuf::from(parts[2]);
                                let is_removable = parts[3].parse::<bool>().unwrap_or(true);
                                let shortcut_hint = if parts.len() >= 5 && !parts[4].is_empty() {
                                    Some(parts[4].to_string())
                                } else {
                                    None
                                };
                                if p.is_dir() || crate::fs::operations::is_trash_path(&p) || name == "Trash" {
                                    loaded.push(Bookmark {
                                        name,
                                        icon,
                                        path: p,
                                        shortcut_hint,
                                        is_removable,
                                    });
                                }
                            }
                        } else {
                            let p = PathBuf::from(line);
                            if p.is_dir() {
                                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("Carpeta").to_string();
                                loaded.push(Bookmark {
                                    name,
                                    icon: "folder-symbolic".to_string(),
                                    path: p,
                                    shortcut_hint: None,
                                    is_removable: true,
                                });
                            }
                        }
                    }
                }
            }
        }

        if !loaded.is_empty() {
            let has_trash = loaded.iter().any(|b| b.name == "Trash" || crate::fs::operations::is_trash_path(&b.path));
            if !has_trash {
                let trash_dir = crate::fs::operations::trash_path();
                loaded.push(Bookmark {
                    name: "Trash".to_string(),
                    icon: "user-trash-symbolic".to_string(),
                    path: trash_dir,
                    shortcut_hint: None,
                    is_removable: false,
                });
            }
            return loaded;
        }

        let mut initial_bookmarks = Vec::new();
        if let Some(home) = dirs::home_dir() {
            initial_bookmarks.push(Bookmark {
                name: "Home".to_string(),
                icon: "user-home-symbolic".to_string(),
                path: home.clone(),
                shortcut_hint: Some("~".to_string()),
                is_removable: false,
            });
            let proj_path = home.join("Documents/Proyects");
            if proj_path.exists() {
                initial_bookmarks.push(Bookmark {
                    name: "Projects".to_string(),
                    icon: "folder-symbolic".to_string(),
                    path: proj_path,
                    shortcut_hint: None,
                    is_removable: true,
                });
            }
        }
        if let Some(p) = dirs::download_dir() {
            initial_bookmarks.push(Bookmark {
                name: "Downloads".to_string(),
                icon: "folder-download-symbolic".to_string(),
                path: p,
                shortcut_hint: None,
                is_removable: false,
            });
        }
        if let Some(p) = dirs::document_dir() {
            initial_bookmarks.push(Bookmark {
                name: "Documents".to_string(),
                icon: "folder-documents-symbolic".to_string(),
                path: p,
                shortcut_hint: None,
                is_removable: false,
            });
        }
        if let Some(p) = dirs::picture_dir() {
            initial_bookmarks.push(Bookmark {
                name: "Pictures".to_string(),
                icon: "folder-pictures-symbolic".to_string(),
                path: p,
                shortcut_hint: None,
                is_removable: false,
            });
        }
        if let Some(p) = dirs::video_dir() {
            initial_bookmarks.push(Bookmark {
                name: "Videos".to_string(),
                icon: "folder-videos-symbolic".to_string(),
                path: p,
                shortcut_hint: None,
                is_removable: false,
            });
        }
        if let Some(p) = dirs::audio_dir() {
            initial_bookmarks.push(Bookmark {
                name: "Music".to_string(),
                icon: "folder-music-symbolic".to_string(),
                path: p,
                shortcut_hint: None,
                is_removable: false,
            });
        }

        if let Some(gtk_bm) = dirs::config_dir().map(|p| p.join("gtk-3.0").join("bookmarks")) {
            if gtk_bm.exists() {
                if let Ok(content) = std::fs::read_to_string(&gtk_bm) {
                    for line in content.lines() {
                        let line = line.trim();
                        if line.starts_with("file://") {
                            let uri = line.split_whitespace().next().unwrap_or("");
                            if let Ok((p, _)) = glib::filename_from_uri(uri) {
                                if p.is_dir() && !initial_bookmarks.iter().any(|b| b.path == p) {
                                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("Carpeta").to_string();
                                    initial_bookmarks.push(Bookmark {
                                        name,
                                        icon: "folder-symbolic".to_string(),
                                        path: p,
                                        shortcut_hint: None,
                                        is_removable: true,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        let trash_dir = crate::fs::operations::trash_path();
        initial_bookmarks.push(Bookmark {
            name: "Trash".to_string(),
            icon: "user-trash-symbolic".to_string(),
            path: trash_dir,
            shortcut_hint: None,
            is_removable: false,
        });

        initial_bookmarks
    }

    fn save_custom_bookmarks(&self) {
        if let Some(path) = Self::bookmarks_config_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let bms = self.bookmarks.borrow();
            let mut lines = Vec::new();
            for b in bms.iter() {
                let hint = b.shortcut_hint.as_deref().unwrap_or("");
                let line = format!(
                    "{}|{}|{}|{}|{}",
                    b.name,
                    b.icon,
                    b.path.to_string_lossy(),
                    b.is_removable,
                    hint
                );
                lines.push(line);
            }
            let _ = std::fs::write(path, lines.join("\n"));
        }
    }

    fn build_bookmark_rows(&self) {
        while let Some(row) = self.bookmarks_list.row_at_index(0) {
            self.bookmarks_list.remove(&row);
        }
        self.bookmark_labels.borrow_mut().clear();
        self.bookmark_hints.borrow_mut().clear();
        self.bookmark_boxes.borrow_mut().clear();

        let bms = self.bookmarks.borrow().clone();
        let is_coll = *self.is_collapsed.borrow();

        for (idx, bm) in bms.iter().enumerate() {
            let row = ListBoxRow::new();
            row.set_focusable(true);
            row.set_activatable(true);
            row.set_widget_name(&format!("{idx}"));
            row.set_tooltip_text(Some(&bm.name));

            let row_box = Box::builder()
                .orientation(Orientation::Horizontal)
                .spacing(6)
                .margin_top(2)
                .margin_bottom(2)
                .margin_start(4)
                .margin_end(4)
                .build();

            let icon = gtk4::Image::builder()
                .icon_name(&bm.icon)
                .pixel_size(16)
                .build();

            let label = Label::builder()
                .label(&bm.name)
                .halign(Align::Start)
                .hexpand(true)
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .build();

            row_box.append(&icon);
            row_box.append(&label);

            if let Some(ref hint) = bm.shortcut_hint {
                let hint_lbl = Label::builder()
                    .label(hint)
                    .css_classes(["dim-label", "caption", "monospace"])
                    .build();
                row_box.append(&hint_lbl);
                if is_coll {
                    hint_lbl.set_visible(false);
                }
                self.bookmark_hints.borrow_mut().push(hint_lbl);
            }

            if is_coll {
                row_box.set_halign(Align::Center);
                label.set_visible(false);
            } else {
                row_box.set_halign(Align::Fill);
                label.set_visible(true);
            }

            self.bookmark_labels.borrow_mut().push(label);
            self.bookmark_boxes.borrow_mut().push(row_box.clone());

            // Drag Source: permite reordenar el elemento al sostenerlo y arrastrarlo
            let drag_source = gtk4::DragSource::new();
            drag_source.set_actions(gdk::DragAction::all());
            let reorder_src_rc = self.reorder_source_idx.clone();
            let row_idx = idx;
            let bm_path = bm.path.clone();
            let row_box_for_icon = row_box.clone();

            drag_source.connect_prepare(move |_, _, _| {
                *reorder_src_rc.borrow_mut() = Some(row_idx);
                let gfile = gio::File::for_path(&bm_path);
                let file_list = gdk::FileList::from_array(&[gfile]);
                Some(gdk::ContentProvider::for_value(&file_list.to_value()))
            });

            let row_weak_drag = row.downgrade();
            let reorder_end_rc = self.reorder_source_idx.clone();
            drag_source.connect_drag_end(move |_, _, _| {
                *reorder_end_rc.borrow_mut() = None;
                if let Some(r) = row_weak_drag.upgrade() {
                    r.remove_css_class("reorder-dragging");
                }
            });

            let row_weak_begin = row.downgrade();
            drag_source.connect_drag_begin(move |ds, _| {
                if let Some(r) = row_weak_begin.upgrade() {
                    r.add_css_class("reorder-dragging");
                }
                let paintable = gtk4::WidgetPaintable::new(Some(&row_box_for_icon));
                ds.set_icon(Some(&paintable), 12, 12);
            });

            row.add_controller(drag_source);

            // Drop Target: detecta posición superior/inferior para insertar antes o después
            let drop_target = gtk4::DropTarget::new(
                gdk::FileList::static_type(),
                gdk::DragAction::all(),
            );

            let row_weak_motion = row.downgrade();
            drop_target.connect_motion(move |_, _x, y| {
                if let Some(r) = row_weak_motion.upgrade() {
                    let height = r.height() as f64;
                    let mid = if height > 0.0 { height / 2.0 } else { 16.0 };
                    if y < mid {
                        r.add_css_class("reorder-drop-top");
                        r.remove_css_class("reorder-drop-bottom");
                    } else {
                        r.add_css_class("reorder-drop-bottom");
                        r.remove_css_class("reorder-drop-top");
                    }
                }
                gdk::DragAction::MOVE
            });

            let row_weak_enter = row.downgrade();
            drop_target.connect_enter(move |_, _x, y| {
                if let Some(r) = row_weak_enter.upgrade() {
                    let height = r.height() as f64;
                    let mid = if height > 0.0 { height / 2.0 } else { 16.0 };
                    if y < mid {
                        r.add_css_class("reorder-drop-top");
                        r.remove_css_class("reorder-drop-bottom");
                    } else {
                        r.add_css_class("reorder-drop-bottom");
                        r.remove_css_class("reorder-drop-top");
                    }
                }
                gdk::DragAction::MOVE
            });

            let row_weak_leave = row.downgrade();
            drop_target.connect_leave(move |_| {
                if let Some(r) = row_weak_leave.upgrade() {
                    r.remove_css_class("reorder-drop-top");
                    r.remove_css_class("reorder-drop-bottom");
                }
            });

            let row_weak_drop = row.downgrade();
            let sb_row = self.clone();
            let target_idx = idx;
            drop_target.connect_drop(move |_, val, _x, y| {
                if let Some(r) = row_weak_drop.upgrade() {
                    r.remove_css_class("reorder-drop-top");
                    r.remove_css_class("reorder-drop-bottom");
                }

                // 1. Reordenamiento interno de marcadores
                if let Some(src_idx) = sb_row.reorder_source_idx.borrow_mut().take() {
                    let height = if let Some(r) = row_weak_drop.upgrade() {
                        r.height() as f64
                    } else {
                        32.0
                    };
                    let mid = if height > 0.0 { height / 2.0 } else { 16.0 };
                    let insert_after = y >= mid;
                    sb_row.reorder_bookmark(src_idx, target_idx, insert_after);
                    return true;
                }

                // 2. Anclar carpeta desde la vista de archivos o externa
                if let Ok(file_list) = val.get::<gdk::FileList>() {
                    let mut added_any = false;
                    for file in file_list.files() {
                        if let Some(path) = file.path() {
                            let is_dir = path.is_dir()
                                || std::fs::metadata(&path).map(|m| m.is_dir()).unwrap_or(false);
                            if is_dir && sb_row.pin_folder_at(path, target_idx, y >= 16.0) {
                                added_any = true;
                            }
                        }
                    }
                    return added_any;
                }

                false
            });

            row.add_controller(drop_target);

            row.set_child(Some(&row_box));
            self.bookmarks_list.append(&row);
        }
    }

    pub fn reorder_bookmark(&self, src_idx: usize, target_idx: usize, insert_after: bool) {
        let mut bms = self.bookmarks.borrow_mut();
        let len = bms.len();
        let Some(new_idx) = compute_reorder_index(src_idx, target_idx, insert_after, len) else {
            return;
        };

        let item = bms.remove(src_idx);
        bms.insert(new_idx, item);
        drop(bms);

        self.save_custom_bookmarks();
        self.build_bookmark_rows();
    }

    pub fn pin_folder_at(&self, path: PathBuf, target_idx: usize, insert_after: bool) -> bool {
        let is_dir = path.is_dir()
            || std::fs::metadata(&path).map(|m| m.is_dir()).unwrap_or(false);
        if !is_dir {
            return false;
        }

        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.clone());

        // Evitar duplicados
        if self.bookmarks.borrow().iter().any(|b| {
            b.path == path || b.path.canonicalize().unwrap_or_else(|_| b.path.clone()) == canonical_path
        }) {
            return false;
        }

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Carpeta")
            .to_string();

        let new_bm = Bookmark {
            name,
            icon: "folder-symbolic".to_string(),
            path,
            shortcut_hint: None,
            is_removable: true,
        };

        let mut bms = self.bookmarks.borrow_mut();
        let mut insert_pos = if target_idx < bms.len() {
            if insert_after { target_idx + 1 } else { target_idx }
        } else {
            bms.len()
        };

        if insert_pos > bms.len() {
            insert_pos = bms.len();
        }

        bms.insert(insert_pos, new_bm);
        drop(bms);

        self.save_custom_bookmarks();
        self.build_bookmark_rows();
        true
    }

    pub fn pin_folder(&self, path: PathBuf) -> bool {
        let trash_idx = self.bookmarks.borrow()
            .iter()
            .position(|b| b.name == "Trash" || crate::fs::operations::is_trash_path(&b.path));
        let pos = trash_idx.unwrap_or_else(|| self.bookmarks.borrow().len());
        self.pin_folder_at(path, pos, false)
    }

    pub fn unpin_path(&self, path: &Path) {
        let mut bms = self.bookmarks.borrow_mut();
        if let Some(pos) = bms.iter().position(|b| b.path == path && b.is_removable) {
            bms.remove(pos);
            drop(bms);
            self.save_custom_bookmarks();
            self.build_bookmark_rows();
        }
    }

    #[allow(dead_code)]
    pub fn unpin_bookmark(&self, index: usize) {
        let mut bms = self.bookmarks.borrow_mut();
        if index < bms.len() && bms[index].is_removable {
            bms.remove(index);
            drop(bms);
            self.save_custom_bookmarks();
            self.build_bookmark_rows();
        }
    }

    fn create_folder_drop_target(&self) -> gtk4::DropTarget {
        let drop_target = gtk4::DropTarget::new(
            gdk::FileList::static_type(),
            gdk::DragAction::all(),
        );
        let sb = self.clone();
        drop_target.connect_drop(move |_, val, _, _| {
            // Si viene de arrastrar un marcador y se suelta en espacio vacío de la barra: mover al final
            if let Some(src_idx) = sb.reorder_source_idx.borrow_mut().take() {
                let last_idx = sb.bookmarks.borrow().len().saturating_sub(1);
                sb.reorder_bookmark(src_idx, last_idx, true);
                return true;
            }

            if let Ok(file_list) = val.get::<gdk::FileList>() {
                let mut added_any = false;
                for file in file_list.files() {
                    if let Some(path) = file.path() {
                        let is_dir = path.is_dir()
                            || std::fs::metadata(&path).map(|m| m.is_dir()).unwrap_or(false);
                        if is_dir && sb.pin_folder(path) {
                            added_any = true;
                        }
                    }
                }
                return added_any;
            }
            false
        });
        drop_target
    }

    fn setup_drag_and_drop(&self) {
        self.bookmarks_list.add_controller(self.create_folder_drop_target());
        self.container.add_controller(self.create_folder_drop_target());
        if let Some(child) = self.container.child() {
            child.add_controller(self.create_folder_drop_target());
        }
    }

    fn build_device_rows(&self) {
        // Limpiar filas anteriores
        while let Some(row) = self.devices_list.row_at_index(0) {
            self.devices_list.remove(&row);
        }
        self.device_widgets.borrow_mut().clear();

        let dev_list = self.devices.borrow().clone();
        let is_coll = *self.is_collapsed.borrow();

        for (idx, dev) in dev_list.iter().enumerate() {
            let row = ListBoxRow::new();
            row.set_focusable(true);
            row.set_activatable(true);
            row.set_widget_name(&format!("dev_{idx}"));

            let free_str = if let Some(ref space) = dev.disk_space {
                format!("{} free", space.formatted_available)
            } else if !dev.is_mounted {
                "Not mounted".to_string()
            } else {
                "—".to_string()
            };

            let tooltip = format!("{} ({})", dev.name, free_str);
            row.set_tooltip_text(Some(&tooltip));

            let main_box = Box::builder()
                .orientation(Orientation::Vertical)
                .spacing(2)
                .margin_top(2)
                .margin_bottom(2)
                .margin_start(4)
                .margin_end(4)
                .build();

            let top_row = Box::builder()
                .orientation(Orientation::Horizontal)
                .spacing(6)
                .build();

            let icon = gtk4::Image::builder()
                .icon_name(dev.icon_name)
                .pixel_size(16)
                .build();

            let name_lbl = Label::builder()
                .label(&dev.name)
                .halign(Align::Start)
                .hexpand(true)
                .css_classes(["caption"])
                .build();

            top_row.append(&icon);
            top_row.append(&name_lbl);

            let (free_lbl_opt, prog_opt) = if let Some(ref space) = dev.disk_space {
                let free_lbl = Label::builder()
                    .label(&space.formatted_available)
                    .halign(Align::End)
                    .css_classes(["caption", "dim-label", "numeric"])
                    .build();
                top_row.append(&free_lbl);

                let progress = ProgressBar::builder()
                    .fraction((space.percent_used / 100.0).clamp(0.0, 1.0))
                    .css_classes(["disk-progress"])
                    .build();

                main_box.append(&top_row);
                main_box.append(&progress);

                (Some(free_lbl), Some(progress))
            } else {
                main_box.append(&top_row);
                (None, None)
            };

            if is_coll {
                main_box.set_halign(Align::Center);
                name_lbl.set_visible(false);
                if let Some(ref fl) = free_lbl_opt {
                    fl.set_visible(false);
                }
                if let Some(ref pr) = prog_opt {
                    pr.set_visible(false);
                }
            } else {
                main_box.set_halign(Align::Fill);
                name_lbl.set_visible(true);
                if let Some(ref fl) = free_lbl_opt {
                    fl.set_visible(true);
                }
                if let Some(ref pr) = prog_opt {
                    pr.set_visible(true);
                }
            }

            self.device_widgets.borrow_mut().push((main_box.clone(), name_lbl, free_lbl_opt, prog_opt));

            row.set_child(Some(&main_box));
            self.devices_list.append(&row);
        }
    }

    fn setup_signals(&self) {
        let bookmarks = self.bookmarks.clone();
        let nav_cb = self.on_navigate_cb.clone();
        self.bookmarks_list.connect_row_activated(move |_, row| {
            let idx = row.index();
            let bms = bookmarks.borrow();
            if idx >= 0 && (idx as usize) < bms.len() {
                let path = bms[idx as usize].path.clone();
                drop(bms);
                if let Some(ref cb) = *nav_cb.borrow() {
                    cb(path);
                }
            }
        });

        // Menú contextual para marcadores (especialmente Vaciar papelera)
        let right_click = gtk4::GestureClick::new();
        right_click.set_button(gtk4::gdk::BUTTON_SECONDARY);
        let bm_list_rc = self.bookmarks_list.clone();
        let bms_rc = self.bookmarks.clone();
        let empty_rc = self.on_empty_trash_cb.clone();
        let popover_c = self.bookmark_popover.clone();
        let vbox_c = self.bookmark_menu_box.clone();

        let sidebar_for_signals = self.clone();
        right_click.connect_pressed(move |_gesture, _, x, y| {
            if let Some(target) = bm_list_rc.pick(x, y, gtk4::PickFlags::DEFAULT) {
                let row_opt = if target.type_() == gtk4::ListBoxRow::static_type() {
                    target.clone().downcast::<gtk4::ListBoxRow>().ok()
                } else {
                    target.ancestor(gtk4::ListBoxRow::static_type()).and_then(|w| w.downcast::<gtk4::ListBoxRow>().ok())
                };

                if let Some(row) = row_opt {
                    let idx = row.index();
                    let bms = bms_rc.borrow();
                    if idx >= 0 && (idx as usize) < bms.len() {
                        let bm = bms[idx as usize].clone();
                        drop(bms);
                        let is_trash = crate::fs::operations::is_trash_path(&bm.path) || bm.name == "Trash";

                        while let Some(child) = vbox_c.first_child() {
                            vbox_c.remove(&child);
                        }

                        let mut has_items = false;

                        if is_trash {
                            let p2 = popover_c.clone();
                            let e1 = empty_rc.clone();
                            let btn_empty = Button::builder()
                                .css_classes(["flat", "menu-item-btn", "destructive-action"])
                                .build();
                            let h_empty = Box::builder()
                                .orientation(Orientation::Horizontal)
                                .spacing(8)
                                .margin_start(4)
                                .margin_end(4)
                                .margin_top(1)
                                .margin_bottom(1)
                                .build();
                            let icon_empty = Image::builder()
                                .icon_name("user-trash-full-symbolic")
                                .pixel_size(15)
                                .build();
                            let lbl_empty = Label::builder()
                                .label("Vaciar papelera")
                                .halign(Align::Start)
                                .hexpand(true)
                                .build();
                            h_empty.append(&icon_empty);
                            h_empty.append(&lbl_empty);
                            btn_empty.set_child(Some(&h_empty));
                            btn_empty.connect_clicked(move |_| {
                                p2.popdown();
                                if let Some(cb) = e1.borrow().as_ref() {
                                    cb();
                                }
                            });
                            vbox_c.append(&btn_empty);
                            has_items = true;
                        } else if bm.is_removable {
                            let p2 = popover_c.clone();
                            let sb = sidebar_for_signals.clone();
                            let remove_path = bm.path.clone();
                            let btn_unpin = Button::builder()
                                .css_classes(["flat", "menu-item-btn", "destructive-action"])
                                .build();
                            let h_unpin = Box::builder()
                                .orientation(Orientation::Horizontal)
                                .spacing(8)
                                .margin_start(4)
                                .margin_end(4)
                                .margin_top(1)
                                .margin_bottom(1)
                                .build();
                            let icon_unpin = Image::builder()
                                .icon_name("list-remove-symbolic")
                                .pixel_size(15)
                                .build();
                            let lbl_unpin = Label::builder()
                                .label("Desanclar de la barra lateral")
                                .halign(Align::Start)
                                .hexpand(true)
                                .build();
                            h_unpin.append(&icon_unpin);
                            h_unpin.append(&lbl_unpin);
                            btn_unpin.set_child(Some(&h_unpin));
                            btn_unpin.connect_clicked(move |_| {
                                p2.popdown();
                                sb.unpin_path(&remove_path);
                            });
                            vbox_c.append(&btn_unpin);
                            has_items = true;
                        }

                        if has_items {
                            let rect = gtk4::gdk::Rectangle::new(x as i32, y as i32, 1, 1);
                            popover_c.set_pointing_to(Some(&rect));
                            popover_c.popup();
                        }
                    }
                }
            }
        });
        self.bookmarks_list.add_controller(right_click);

        let devices_ref = self.devices.clone();
        let nav_cb_dev = self.on_navigate_cb.clone();
        self.devices_list.connect_row_activated(move |_, row| {
            let idx = row.index();
            if idx >= 0 {
                let dev_opt = devices_ref.borrow().get(idx as usize).cloned();
                if let Some(dev) = dev_opt {
                    if let Some(ref path) = dev.path {
                        if let Some(ref cb) = *nav_cb_dev.borrow() {
                            cb(path.clone());
                        }
                    } else if let Some(ref vol) = dev.volume {
                        let vol_clone = vol.clone();
                        let nav_inner = nav_cb_dev.clone();
                        vol.mount(
                            gio::MountMountFlags::NONE,
                            Option::<&gio::MountOperation>::None,
                            gio::Cancellable::NONE,
                            move |res| {
                                if res.is_ok() {
                                    if let Some(m) = vol_clone.get_mount() {
                                        if let Some(p) = m.root().path() {
                                            if let Some(ref cb) = *nav_inner.borrow() {
                                                cb(p);
                                            }
                                        }
                                    }
                                }
                            },
                        );
                    }
                }
            }
        });

        // Controlador de teclado para navegación fluida entre marcadores y dispositivos
        let key_ctrl = EventControllerKey::new();
        let sidebar_for_key = self.clone();
        key_ctrl.connect_key_pressed(move |_, key, _, _| {
            match key {
                Key::Down | Key::j => {
                    sidebar_for_key.select_next();
                    glib::Propagation::Stop
                }
                Key::Up | Key::k => {
                    sidebar_for_key.select_prev();
                    glib::Propagation::Stop
                }
                Key::Return | Key::KP_Enter => {
                    if sidebar_for_key.activate_selected() {
                        glib::Propagation::Stop
                    } else {
                        glib::Propagation::Proceed
                    }
                }
                _ => glib::Propagation::Proceed,
            }
        });
        self.container.add_controller(key_ctrl);
    }

    fn setup_volume_monitor(&self) {
        let dev_ref = self.devices.clone();
        let dev_list_widget = self.devices_list.clone();
        let is_coll = self.is_collapsed.clone();
        let dev_widgets = self.device_widgets.clone();

        let refresh = move || {
            *dev_ref.borrow_mut() = get_storage_devices();
            // Reconstruir
            while let Some(row) = dev_list_widget.row_at_index(0) {
                dev_list_widget.remove(&row);
            }
            dev_widgets.borrow_mut().clear();

            let cur_devs = dev_ref.borrow().clone();
            let collapsed = *is_coll.borrow();

            for (idx, dev) in cur_devs.iter().enumerate() {
                let row = ListBoxRow::new();
                row.set_focusable(true);
                row.set_activatable(true);
                row.set_widget_name(&format!("dev_{idx}"));

                let free_str = if let Some(ref space) = dev.disk_space {
                    format!("{} free", space.formatted_available)
                } else if !dev.is_mounted {
                    "Not mounted".to_string()
                } else {
                    "—".to_string()
                };

                let tooltip = format!("{} ({})", dev.name, free_str);
                row.set_tooltip_text(Some(&tooltip));

                let main_box = Box::builder()
                    .orientation(Orientation::Vertical)
                    .spacing(3)
                    .margin_top(4)
                    .margin_bottom(4)
                    .margin_start(6)
                    .margin_end(6)
                    .build();

                let top_row = Box::builder()
                    .orientation(Orientation::Horizontal)
                    .spacing(8)
                    .build();

                let icon = gtk4::Image::builder()
                    .icon_name(dev.icon_name)
                    .pixel_size(16)
                    .build();

                let name_lbl = Label::builder()
                    .label(&dev.name)
                    .halign(Align::Start)
                    .hexpand(true)
                    .ellipsize(gtk4::pango::EllipsizeMode::End)
                    .css_classes(["caption"])
                    .build();

                top_row.append(&icon);
                top_row.append(&name_lbl);

                let (free_lbl_opt, prog_opt) = if let Some(ref space) = dev.disk_space {
                    let free_lbl = Label::builder()
                        .label(&space.formatted_available)
                        .halign(Align::End)
                        .css_classes(["caption", "dim-label", "numeric"])
                        .build();
                    top_row.append(&free_lbl);

                    let progress = ProgressBar::builder()
                        .fraction((space.percent_used / 100.0).clamp(0.0, 1.0))
                        .css_classes(["disk-progress"])
                        .build();

                    main_box.append(&top_row);
                    main_box.append(&progress);

                    (Some(free_lbl), Some(progress))
                } else {
                    main_box.append(&top_row);
                    (None, None)
                };

                if collapsed {
                    main_box.set_halign(Align::Center);
                    name_lbl.set_visible(false);
                    if let Some(ref fl) = free_lbl_opt {
                        fl.set_visible(false);
                    }
                    if let Some(ref pr) = prog_opt {
                        pr.set_visible(false);
                    }
                } else {
                    main_box.set_halign(Align::Fill);
                    name_lbl.set_visible(true);
                    if let Some(ref fl) = free_lbl_opt {
                        fl.set_visible(true);
                    }
                    if let Some(ref pr) = prog_opt {
                        pr.set_visible(true);
                    }
                }

                dev_widgets.borrow_mut().push((main_box.clone(), name_lbl, free_lbl_opt, prog_opt));
                row.set_child(Some(&main_box));
                dev_list_widget.append(&row);
            }
        };

        let vm = gio::VolumeMonitor::get();
        let r1 = refresh.clone();
        vm.connect_mount_added(move |_, _| r1());
        let r2 = refresh.clone();
        vm.connect_mount_removed(move |_, _| r2());
        let r3 = refresh.clone();
        vm.connect_volume_added(move |_, _| r3());
        let r4 = refresh;
        vm.connect_volume_removed(move |_, _| r4());
    }

    pub fn set_collapsed(&self, collapsed: bool) {
        *self.is_collapsed.borrow_mut() = collapsed;

        if collapsed {
            self.container.set_width_request(48);
            self.bookmarks_header.set_visible(false);
            self.devices_header.set_visible(false);

            for l in self.bookmark_labels.borrow().iter() {
                l.set_visible(false);
            }
            for h in self.bookmark_hints.borrow().iter() {
                h.set_visible(false);
            }
            for b in self.bookmark_boxes.borrow().iter() {
                b.set_halign(Align::Center);
            }

            for (box_w, name_lbl, free_lbl, prog) in self.device_widgets.borrow().iter() {
                box_w.set_halign(Align::Center);
                name_lbl.set_visible(false);
                if let Some(fl) = free_lbl {
                    fl.set_visible(false);
                }
                if let Some(pr) = prog {
                    pr.set_visible(false);
                }
            }
        } else {
            self.container.set_width_request(130);
            self.bookmarks_header.set_visible(true);
            self.devices_header.set_visible(true);

            for l in self.bookmark_labels.borrow().iter() {
                l.set_visible(true);
            }
            for h in self.bookmark_hints.borrow().iter() {
                h.set_visible(true);
            }
            for b in self.bookmark_boxes.borrow().iter() {
                b.set_halign(Align::Fill);
            }

            for (box_w, name_lbl, free_lbl, prog) in self.device_widgets.borrow().iter() {
                box_w.set_halign(Align::Fill);
                name_lbl.set_visible(true);
                if let Some(fl) = free_lbl {
                    fl.set_visible(true);
                }
                if let Some(pr) = prog {
                    pr.set_visible(true);
                }
            }
        }

        let count = *self.active_tasks_count.borrow();
        if count > 0 {
            if collapsed {
                self.op_header.set_visible(false);
                self.op_cards_container.set_visible(false);
                self.collapsed_btn.set_visible(true);
            } else {
                self.popover.popdown();
                self.collapsed_btn.set_visible(false);
                self.op_header.set_visible(true);
                self.op_cards_container.set_visible(true);
            }
        } else {
            self.op_header.set_visible(false);
            self.op_cards_container.set_visible(false);
            self.collapsed_btn.set_visible(false);
        }
    }

    pub fn toggle_progress_popover(&self) {
        let count = *self.active_tasks_count.borrow();
        if count == 0 {
            return;
        }
        let is_coll = *self.is_collapsed.borrow();
        if is_coll {
            if self.popover.is_visible() {
                self.popover.popdown();
            } else {
                self.popover.popup();
            }
        } else {
            self.op_header.set_visible(true);
            self.op_cards_container.set_visible(true);
        }
    }

    pub fn start_operation<F>(
        &self,
        title: &str,
        icon_name: &str,
        on_finish: F,
    ) -> (Sender<ProgressMessage>, Arc<AtomicBool>)
    where
        F: Fn(bool, Option<PathBuf>) + 'static,
    {
        let (sender, receiver): (Sender<ProgressMessage>, Receiver<ProgressMessage>) = channel();
        let cancel_flag = Arc::new(AtomicBool::new(false));

        let count = {
            let mut c = self.active_tasks_count.borrow_mut();
            *c += 1;
            *c
        };

        // 1. Tarjeta para la barra lateral expandida (diseño idéntico a discos de dispositivos)
        let card = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(3)
            .margin_start(6)
            .margin_end(6)
            .margin_top(4)
            .margin_bottom(4)
            .css_classes(["sidebar-op-card"])
            .build();

        let top_row = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(6)
            .build();

        let icon = Image::builder()
            .icon_name(icon_name)
            .pixel_size(16)
            .build();

        let title_lbl = Label::builder()
            .label(title)
            .halign(Align::Start)
            .hexpand(true)
            .css_classes(["caption", "bold"])
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();

        let percent_lbl = Label::builder()
            .label("0%")
            .halign(Align::End)
            .css_classes(["caption", "dim-label", "numeric"])
            .build();

        let btn_cancel = Button::builder()
            .icon_name("window-close-symbolic")
            .css_classes(["flat", "circular"])
            .tooltip_text("Cancel operation")
            .build();

        top_row.append(&icon);
        top_row.append(&title_lbl);
        top_row.append(&percent_lbl);
        top_row.append(&btn_cancel);

        let detail_lbl = Label::builder()
            .label("Starting...")
            .halign(Align::Start)
            .css_classes(["caption", "dim-label"])
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();

        let progress_bar = ProgressBar::builder()
            .fraction(0.0)
            .css_classes(["disk-progress"])
            .build();

        let visto_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .halign(Align::End)
            .spacing(4)
            .margin_top(2)
            .build();

        let btn_visto = Button::builder()
            .label("Cerrar")
            .css_classes(["suggested-action", "compact-btn"])
            .visible(false)
            .build();

        visto_box.append(&btn_visto);

        card.append(&top_row);
        card.append(&detail_lbl);
        card.append(&progress_bar);
        card.append(&visto_box);

        // 2. Tarjeta para la ventana flotante Popover (modo comprimido)
        let pop_card = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(3)
            .margin_bottom(6)
            .css_classes(["sidebar-op-card"])
            .build();

        let pop_top_row = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(6)
            .build();

        let pop_icon = Image::builder()
            .icon_name(icon_name)
            .pixel_size(16)
            .build();

        let pop_title_lbl = Label::builder()
            .label(title)
            .halign(Align::Start)
            .hexpand(true)
            .css_classes(["caption", "bold"])
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();

        let pop_percent_lbl = Label::builder()
            .label("0%")
            .halign(Align::End)
            .css_classes(["caption", "dim-label", "numeric"])
            .build();

        let pop_btn_cancel = Button::builder()
            .icon_name("window-close-symbolic")
            .css_classes(["flat", "circular"])
            .tooltip_text("Cancel operation")
            .build();

        pop_top_row.append(&pop_icon);
        pop_top_row.append(&pop_title_lbl);
        pop_top_row.append(&pop_percent_lbl);
        pop_top_row.append(&pop_btn_cancel);

        let pop_detail_lbl = Label::builder()
            .label("Starting...")
            .halign(Align::Start)
            .css_classes(["caption", "dim-label"])
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();

        let pop_progress_bar = ProgressBar::builder()
            .fraction(0.0)
            .css_classes(["disk-progress"])
            .build();

        let pop_visto_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .halign(Align::End)
            .spacing(4)
            .margin_top(2)
            .build();

        let pop_btn_visto = Button::builder()
            .label("Cerrar")
            .css_classes(["suggested-action", "compact-btn"])
            .visible(false)
            .build();

        pop_visto_box.append(&pop_btn_visto);

        pop_card.append(&pop_top_row);
        pop_card.append(&pop_detail_lbl);
        pop_card.append(&pop_progress_bar);
        pop_card.append(&pop_visto_box);

        self.op_cards_container.append(&card);
        self.pop_cards_container.append(&pop_card);

        // Actualizar visibilidad según el estado de la barra lateral
        let is_coll = *self.is_collapsed.borrow();
        if is_coll {
            self.op_header.set_visible(false);
            self.op_cards_container.set_visible(false);
            self.collapsed_btn.set_visible(true);
        } else {
            self.popover.popdown();
            self.collapsed_btn.set_visible(false);
            self.op_header.set_visible(true);
            self.op_cards_container.set_visible(true);
        }

        let tip = if count == 1 {
            "1 operation in progress (Press 'b' or hover)".to_string()
        } else {
            format!("{count} operations in progress (Press 'b' or hover)")
        };
        self.collapsed_btn.set_tooltip_text(Some(&tip));

        // Hook de cancelación para este proceso
        let cancel_flag_c = cancel_flag.clone();
        let btn_c = btn_cancel.clone();
        let pop_btn_c = pop_btn_cancel.clone();
        let d_lbl = detail_lbl.clone();
        let pop_d_lbl = pop_detail_lbl.clone();
        let t_lbl = title_lbl.clone();
        let pop_t_lbl = pop_title_lbl.clone();

        let do_cancel = move || {
            cancel_flag_c.store(true, Ordering::Relaxed);
            btn_c.set_sensitive(false);
            pop_btn_c.set_sensitive(false);
            t_lbl.set_text("Cancelling...");
            pop_t_lbl.set_text("Cancelling...");
            d_lbl.set_text("Stopping operation and cleaning up...");
            pop_d_lbl.set_text("Stopping operation and cleaning up...");
        };

        let dc1 = do_cancel.clone();
        btn_cancel.connect_clicked(move |_| dc1());
        let dc2 = do_cancel;
        pop_btn_cancel.connect_clicked(move |_| dc2());

        // Almacenamiento de estado final
        let last_result_path = Rc::new(RefCell::new(None));
        let success_flag = Rc::new(RefCell::new(false));
        let on_finish_rc = Rc::new(RefCell::new(Some(on_finish)));

        // Hook de visto para este proceso
        let op_cards_c = self.op_cards_container.clone();
        let pop_cards_c = self.pop_cards_container.clone();
        let card_c = card.clone();
        let pop_card_c = pop_card.clone();
        let op_header_c = self.op_header.clone();
        let collapsed_btn_c = self.collapsed_btn.clone();
        let popover_c = self.popover.clone();
        let tasks_count_c = self.active_tasks_count.clone();
        let res_path_c = last_result_path.clone();
        let succ_c = success_flag.clone();
        let on_finish_c = on_finish_rc.clone();

        let executed = Rc::new(RefCell::new(false));
        let do_visto = {
            let executed = executed.clone();
            move || {
                if *executed.borrow() {
                    return;
                }
                *executed.borrow_mut() = true;
                op_cards_c.remove(&card_c);
                pop_cards_c.remove(&pop_card_c);

                let remaining = {
                    let mut c = tasks_count_c.borrow_mut();
                    *c = c.saturating_sub(1);
                    *c
                };

                if remaining == 0 {
                    op_header_c.set_visible(false);
                    op_cards_c.set_visible(false);
                    collapsed_btn_c.set_visible(false);
                    popover_c.popdown();
                } else {
                    let tip = if remaining == 1 {
                        "1 operation in progress (Press 'b' or hover)".to_string()
                    } else {
                        format!("{remaining} operations in progress (Press 'b' or hover)")
                    };
                    collapsed_btn_c.set_tooltip_text(Some(&tip));
                }

                let cb_opt = on_finish_c.borrow_mut().take();
                if let Some(cb) = cb_opt {
                    let succ = *succ_c.borrow();
                    let path = res_path_c.borrow().clone();
                    cb(succ, path);
                }
            }
        };

        let dv1 = do_visto.clone();
        btn_visto.connect_clicked(move |_| dv1());
        let dv2 = do_visto.clone();
        pop_btn_visto.connect_clicked(move |_| dv2());

        // Bucle de temporización independiente para este proceso
        let icon_c = icon.clone();
        let pop_icon_c = pop_icon.clone();
        let title_c = title_lbl.clone();
        let pop_title_c = pop_title_lbl.clone();
        let pct_c = percent_lbl.clone();
        let pop_pct_c = pop_percent_lbl.clone();
        let detail_c = detail_lbl.clone();
        let pop_detail_c = pop_detail_lbl.clone();
        let bar_c = progress_bar.clone();
        let pop_bar_c = pop_progress_bar.clone();
        let cancel_c = btn_cancel.clone();
        let pop_cancel_c = pop_btn_cancel.clone();
        let visto_c = btn_visto.clone();
        let pop_visto_c = pop_btn_visto.clone();
        let res_store = last_result_path.clone();
        let succ_store = success_flag.clone();
        let on_finish_timeout = on_finish_rc.clone();

        glib::timeout_add_local(Duration::from_millis(25), move || {
            let mut stop = false;
            while let Ok(msg) = receiver.try_recv() {
                match msg {
                    ProgressMessage::Progress { fraction, status, sub_status } => {
                        let clamped = fraction.clamp(0.0, 1.0);
                        bar_c.set_fraction(clamped);
                        pop_bar_c.set_fraction(clamped);

                        let pct_str = format!("{:.0}%", clamped * 100.0);
                        pct_c.set_text(&pct_str);
                        pop_pct_c.set_text(&pct_str);

                        if !status.is_empty() {
                            title_c.set_text(&status);
                            pop_title_c.set_text(&status);
                        }
                        if !sub_status.is_empty() {
                            detail_c.set_text(&sub_status);
                            pop_detail_c.set_text(&sub_status);
                        }
                    }
                    ProgressMessage::Success { title: t, message: m, result_path } => {
                        bar_c.set_fraction(1.0);
                        pop_bar_c.set_fraction(1.0);
                        pct_c.set_text("100%");
                        pop_pct_c.set_text("100%");

                        icon_c.set_icon_name(Some("object-select-symbolic"));
                        pop_icon_c.set_icon_name(Some("object-select-symbolic"));

                        title_c.set_text("Completado");
                        pop_title_c.set_text("Completado");
                        detail_c.set_text(&m);
                        pop_detail_c.set_text(&m);

                        cancel_c.set_visible(false);
                        pop_cancel_c.set_visible(false);
                        visto_c.set_visible(true);
                        pop_visto_c.set_visible(true);

                        *res_store.borrow_mut() = result_path.clone();
                        *succ_store.borrow_mut() = true;

                        let cb_opt = on_finish_timeout.borrow_mut().take();
                        if let Some(cb) = cb_opt {
                            cb(true, result_path);
                        }

                        send_system_notification(&t, &m, false);

                        // Auto-cierre discreto tras 4 segundos
                        let dv_auto = do_visto.clone();
                        glib::timeout_add_seconds_local(4, move || {
                            dv_auto();
                            glib::ControlFlow::Break
                        });

                        stop = true;
                    }
                    ProgressMessage::Error { message: m } => {
                        icon_c.set_icon_name(Some("dialog-error-symbolic"));
                        pop_icon_c.set_icon_name(Some("dialog-error-symbolic"));

                        title_c.set_text("Error");
                        pop_title_c.set_text("Error en la operación");
                        detail_c.set_text(&m);
                        pop_detail_c.set_text(&m);

                        cancel_c.set_visible(false);
                        pop_cancel_c.set_visible(false);
                        visto_c.set_visible(true);
                        pop_visto_c.set_visible(true);

                        *succ_store.borrow_mut() = false;

                        let cb_opt = on_finish_timeout.borrow_mut().take();
                        if let Some(cb) = cb_opt {
                            cb(false, None);
                        }

                        send_system_notification("Error en la operación", &m, true);
                        stop = true;
                    }
                    ProgressMessage::Cancelled => {
                        icon_c.set_icon_name(Some("dialog-warning-symbolic"));
                        pop_icon_c.set_icon_name(Some("dialog-warning-symbolic"));

                        title_c.set_text("Cancelado");
                        pop_title_c.set_text("Operación cancelada");
                        detail_c.set_text("La tarea fue interrumpida.");
                        pop_detail_c.set_text("La tarea fue interrumpida.");

                        cancel_c.set_visible(false);
                        pop_cancel_c.set_visible(false);
                        visto_c.set_visible(true);
                        pop_visto_c.set_visible(true);

                        *succ_store.borrow_mut() = false;

                        let cb_opt = on_finish_timeout.borrow_mut().take();
                        if let Some(cb) = cb_opt {
                            cb(false, None);
                        }

                        send_system_notification("Operación cancelada", "La tarea fue interrumpida.", true);
                        stop = true;
                    }
                }
            }

            if stop {
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });

        (sender, cancel_flag)
    }

    pub fn toggle_collapse(&self) -> bool {
        let new_state = !*self.is_collapsed.borrow();
        self.set_collapsed(new_state);
        new_state
    }

    #[allow(dead_code)]
    pub fn is_collapsed(&self) -> bool {
        *self.is_collapsed.borrow()
    }

    pub fn update_disk_space(&self, _path: &Path) {
        // Actualiza el estado de los dispositivos
        *self.devices.borrow_mut() = get_storage_devices();
        self.build_device_rows();
    }

    pub fn connect_navigate<F>(&self, on_navigate: F)
    where
        F: Fn(PathBuf) + 'static,
    {
        *self.on_navigate_cb.borrow_mut() = Some(Rc::new(on_navigate));
    }

    pub fn connect_empty_trash<F>(&self, on_empty: F)
    where
        F: Fn() + 'static,
    {
        *self.on_empty_trash_cb.borrow_mut() = Some(Rc::new(on_empty));
    }

    pub fn widget(&self) -> &ScrolledWindow {
        &self.container
    }

    #[allow(dead_code)]
    pub fn select_path(&self, _current: &Path) {
        // Las rutas de la barra lateral son accesos rápidos y no se marcan como activas
    }

    fn focused_bookmark_index(&self) -> Option<usize> {
        let focus_widget = self.container.root().and_then(|r| gtk4::prelude::RootExt::focus(&r));

        let bms_len = self.bookmarks.borrow().len();
        for idx in 0..bms_len {
            if let Some(row) = self.bookmarks_list.row_at_index(idx as i32) {
                if row.has_focus() {
                    return Some(idx);
                }
                if let Some(ref fw) = focus_widget {
                    if fw.is_ancestor(&row) || fw == &row {
                        return Some(idx);
                    }
                }
            }
        }
        None
    }

    fn focused_device_index(&self) -> Option<usize> {
        let focus_widget = self.container.root().and_then(|r| gtk4::prelude::RootExt::focus(&r));

        let dev_len = self.devices.borrow().len();
        for idx in 0..dev_len {
            if let Some(row) = self.devices_list.row_at_index(idx as i32) {
                if row.has_focus() {
                    return Some(idx);
                }
                if let Some(ref fw) = focus_widget {
                    if fw.is_ancestor(&row) || fw == &row {
                        return Some(idx);
                    }
                }
            }
        }
        None
    }

    pub fn select_device(&self, index: usize) {
        if let Some(row) = self.devices_list.row_at_index(index as i32) {
            row.grab_focus();
        }
    }

    pub fn select_bookmark(&self, index: usize) {
        if let Some(row) = self.bookmarks_list.row_at_index(index as i32) {
            row.grab_focus();
        }
    }

    pub fn grab_focus(&self) {
        if let Some(idx) = self.focused_bookmark_index() {
            if let Some(row) = self.bookmarks_list.row_at_index(idx as i32) {
                row.grab_focus();
                return;
            }
        }
        if let Some(first) = self.bookmarks_list.row_at_index(0) {
            first.grab_focus();
        }
    }

    pub fn activate_selected(&self) -> bool {
        if let Some(idx) = self.focused_bookmark_index() {
            let bms = self.bookmarks.borrow();
            if idx < bms.len() {
                let path = bms[idx].path.clone();
                drop(bms);
                if let Some(ref cb) = *self.on_navigate_cb.borrow() {
                    cb(path);
                    return true;
                }
            }
        }
        if let Some(idx) = self.focused_device_index() {
            let dev_opt = self.devices.borrow().get(idx).cloned();
            if let Some(dev) = dev_opt {
                if let Some(ref path) = dev.path {
                    if let Some(ref cb) = *self.on_navigate_cb.borrow() {
                        cb(path.clone());
                        return true;
                    }
                } else if let Some(ref vol) = dev.volume {
                    let vol_clone = vol.clone();
                    let nav_cb = self.on_navigate_cb.clone();
                    vol.mount(
                        gio::MountMountFlags::NONE,
                        Option::<&gio::MountOperation>::None,
                        gio::Cancellable::NONE,
                        move |res| {
                            if res.is_ok() {
                                if let Some(m) = vol_clone.get_mount() {
                                    if let Some(p) = m.root().path() {
                                        if let Some(ref cb) = *nav_cb.borrow() {
                                            cb(p);
                                        }
                                    }
                                }
                            }
                        },
                    );
                    return true;
                }
            }
        }
        false
    }

    pub fn select_next(&self) {
        if let Some(idx) = self.focused_bookmark_index() {
            let bms_len = self.bookmarks.borrow().len();
            if idx + 1 < bms_len {
                self.select_bookmark(idx + 1);
            } else if !self.devices.borrow().is_empty() {
                self.select_device(0);
            }
        } else if let Some(idx) = self.focused_device_index() {
            let dev_count = self.devices.borrow().len();
            if idx + 1 < dev_count {
                self.select_device(idx + 1);
            }
        } else if let Some(first) = self.bookmarks_list.row_at_index(0) {
            first.grab_focus();
        }
    }

    pub fn select_prev(&self) {
        if let Some(idx) = self.focused_device_index() {
            if idx > 0 {
                self.select_device(idx - 1);
            } else {
                let bms_len = self.bookmarks.borrow().len();
                if bms_len > 0 {
                    self.select_bookmark(bms_len - 1);
                }
            }
        } else if let Some(idx) = self.focused_bookmark_index() {
            if idx > 0 {
                self.select_bookmark(idx - 1);
            }
        } else if let Some(first) = self.bookmarks_list.row_at_index(0) {
            first.grab_focus();
        }
    }

    pub fn selected_path(&self) -> Option<PathBuf> {
        if let Some(idx) = self.focused_bookmark_index() {
            return self.bookmarks.borrow().get(idx).map(|b| b.path.clone());
        }
        if let Some(idx) = self.focused_device_index() {
            return self.devices.borrow().get(idx).and_then(|d| d.path.clone());
        }
        None
    }

    pub fn has_focus(&self) -> bool {
        if let Some(fw) = self.container.root().and_then(|r| gtk4::prelude::RootExt::focus(&r)) {
            fw.is_ancestor(&self.container) || fw == self.container
        } else {
            false
        }
    }
}

pub fn compute_reorder_index(src_idx: usize, target_idx: usize, insert_after: bool, total_len: usize) -> Option<usize> {
    if src_idx >= total_len || target_idx >= total_len || src_idx == target_idx {
        return None;
    }
    let mut new_idx = if src_idx < target_idx {
        let adj_target = target_idx - 1;
        if insert_after { adj_target + 1 } else { adj_target }
    } else {
        if insert_after { target_idx + 1 } else { target_idx }
    };
    if new_idx > total_len.saturating_sub(1) {
        new_idx = total_len.saturating_sub(1);
    }
    Some(new_idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_reorder_index_forward() {
        // [A(0), B(1), C(2), D(3), E(4)]
        // Move B(1) to after D(3) -> new index should be 3
        assert_eq!(compute_reorder_index(1, 3, true, 5), Some(3));
        // Move B(1) to before D(3) -> new index should be 2
        assert_eq!(compute_reorder_index(1, 3, false, 5), Some(2));
    }

    #[test]
    fn test_compute_reorder_index_backward() {
        // [A(0), B(1), C(2), D(3), E(4)]
        // Move D(3) to before B(1) -> new index should be 1
        assert_eq!(compute_reorder_index(3, 1, false, 5), Some(1));
        // Move D(3) to after B(1) -> new index should be 2
        assert_eq!(compute_reorder_index(3, 1, true, 5), Some(2));
    }

    #[test]
    fn test_compute_reorder_index_edges() {
        // Move to first position
        assert_eq!(compute_reorder_index(3, 0, false, 5), Some(0));
        // Move to last position
        assert_eq!(compute_reorder_index(0, 4, true, 5), Some(4));
        // Same index does nothing
        assert_eq!(compute_reorder_index(2, 2, false, 5), None);
        assert_eq!(compute_reorder_index(2, 2, true, 5), None);
        // Out of bounds
        assert_eq!(compute_reorder_index(5, 1, false, 5), None);
        assert_eq!(compute_reorder_index(1, 5, false, 5), None);
    }
}

