use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{channel, Sender};
use std::sync::OnceLock;
use std::time::Duration;
use gtk4::gdk;
use gtk4::prelude::*;
use gtk4::{
    Align, Box, DragSource, DropTarget, FlowBox, FlowBoxChild, Label, ListBox, ListBoxRow,
    Orientation, PolicyType, ScrolledWindow, SelectionMode, Stack, StackTransitionType,
};
use crate::fs::entry::FileEntry;

fn thumbnail_pool() -> &'static glib::ThreadPool {
    static POOL: OnceLock<glib::ThreadPool> = OnceLock::new();
    POOL.get_or_init(|| {
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(8)
            .clamp(4, 16);
        glib::ThreadPool::shared(Some(threads as u32)).expect("Failed to create thumbnail thread pool")
    })
}

fn is_media_file(name: &str) -> bool {
    let ext = std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "svg" | "gif" | "bmp" | "ico" | "avif" | "tiff"
        | "mp4" | "mkv" | "avi" | "webm" | "mov" | "flv" | "wmv" | "m4v"
    )
}

fn load_media_thumbnail(path: &PathBuf, target_size: i32) -> Option<Vec<u8>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // 1. PRIMERO: Buscar en caché estándar de Freedesktop (~/.cache/thumbnails/...)
    // ¡Esto es ultra rápido (~0.01ms) porque el archivo ya está renderizado en disco por el sistema!
    let file = gio::File::for_path(path);
    let uri = file.uri();
    if let Some(hash) = glib::compute_checksum_for_string(glib::ChecksumType::Md5, &uri) {
        if let Some(home) = std::env::var_os("HOME") {
            let home_path = PathBuf::from(home);
            for dir_name in &["normal", "large", "x-large", "xx-large"] {
                let thumb_path = home_path.join(".cache/thumbnails").join(dir_name).join(format!("{hash}.png"));
                if thumb_path.exists() {
                    if let Ok(bytes) = std::fs::read(&thumb_path) {
                        return Some(bytes);
                    }
                }
            }
        }
    }

    let is_image = matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "svg" | "gif" | "bmp" | "ico" | "avif" | "tiff"
    );

    if is_image {
        let req_size = (target_size * 2).max(48);
        if let Ok(pixbuf) = gdk_pixbuf::Pixbuf::from_file_at_scale(path, req_size, req_size, true) {
            // Para imágenes sin alfa, JPEG es entre 10 y 20 veces más rápido de comprimir que PNG
            if !pixbuf.has_alpha() {
                if let Ok(jpeg_bytes) = pixbuf.save_to_bufferv("jpeg", &[("quality", "80")]) {
                    return Some(jpeg_bytes);
                }
            }
            if let Ok(png_bytes) = pixbuf.save_to_bufferv("png", &[]) {
                return Some(png_bytes);
            }
        }
    }

    let is_video = matches!(
        ext.as_str(),
        "mp4" | "mkv" | "avi" | "webm" | "mov" | "flv" | "wmv" | "m4v"
    );

    if is_video {
        let scale_filter = format!("scale={}:-1", (target_size * 2).max(64));
        if let Ok(output) = std::process::Command::new("ffmpeg")
            .args([
                "-loglevel", "error",
                "-ss", "00:00:01",
                "-i", path.to_str().unwrap_or(""),
                "-vf", &scale_filter,
                "-vframes", "1",
                "-f", "image2pipe",
                "-vcodec", "mjpeg",
                "-q:v", "5",
                "-"
            ])
            .output()
        {
            if output.status.success() && !output.stdout.is_empty() {
                return Some(output.stdout);
            }
        }
    }

    None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewMode {
    List,
    Grid,
}

#[derive(Clone)]
pub struct FileList {
    container: ScrolledWindow,
    stack: Stack,
    list_box: ListBox,
    flow_box: FlowBox,
    view_mode: Rc<RefCell<ViewMode>>,
    zoom_level: Rc<RefCell<i32>>, // -2..=4 (default 0)
    entries: Rc<RefCell<Vec<FileEntry>>>,
    visible_indices: Rc<RefCell<Vec<usize>>>,
    filter_query: Rc<RefCell<String>>,
    thumbnail_sender: Sender<(PathBuf, Vec<u8>)>,
    thumbnail_map: Rc<RefCell<HashMap<PathBuf, glib::WeakRef<gtk4::Image>>>>,
    texture_cache: Rc<RefCell<HashMap<PathBuf, gdk::Texture>>>,
    on_context_menu: Rc<RefCell<Option<Rc<dyn Fn(bool, f64, f64)>>>>,
}

impl FileList {
    pub fn new<FSelect>(
        on_selection_changed: FSelect,
        is_grid: bool,
        zoom_level: i32,
    ) -> Self
    where
        FSelect: Fn(Vec<FileEntry>) + 'static + Clone,
    {
        let container = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .hexpand(true)
            .vexpand(true)
            .css_classes(["view", "file-list-scroll"])
            .build();

        let stack = Stack::builder()
            .transition_type(StackTransitionType::Crossfade)
            .transition_duration(150)
            .hexpand(true)
            .vexpand(true)
            .build();

        let list_box = ListBox::builder()
            .selection_mode(SelectionMode::Multiple)
            .activate_on_single_click(false)
            .can_focus(true)
            .css_classes(["navigation-sidebar", "rich-list", "file-list"])
            .build();

        let flow_box = FlowBox::builder()
            .selection_mode(SelectionMode::Multiple)
            .activate_on_single_click(false)
            .can_focus(true)
            .valign(Align::Start)
            .max_children_per_line(24)
            .min_children_per_line(4)
            .homogeneous(true)
            .css_classes(["file-grid"])
            .build();

        let initial_mode = if is_grid { ViewMode::Grid } else { ViewMode::List };
        let view_mode = Rc::new(RefCell::new(initial_mode));
        let zoom_level = Rc::new(RefCell::new(zoom_level.clamp(-2, 4)));
        let entries: Rc<RefCell<Vec<FileEntry>>> = Rc::new(RefCell::new(Vec::new()));
        let visible_indices = Rc::new(RefCell::new(Vec::new()));
        let filter_query = Rc::new(RefCell::new(String::new()));

        // Manejo de cambio de selección en Lista
        let entries_sel = entries.clone();
        let on_sel = on_selection_changed.clone();
        list_box.connect_selected_rows_changed(move |lb| {
            let ents = entries_sel.borrow();
            let selected: Vec<FileEntry> = lb
                .selected_rows()
                .into_iter()
                .filter_map(|row| row.widget_name().parse::<usize>().ok())
                .filter_map(|idx| ents.get(idx).cloned())
                .collect();
            on_sel(selected);
        });

        // Manejo de cambio de selección en Cuadrícula
        let entries_sel2 = entries.clone();
        let on_sel2 = on_selection_changed;
        flow_box.connect_selected_children_changed(move |fb| {
            let ents = entries_sel2.borrow();
            let selected: Vec<FileEntry> = fb
                .selected_children()
                .into_iter()
                .filter_map(|child| child.widget_name().parse::<usize>().ok())
                .filter_map(|idx| ents.get(idx).cloned())
                .collect();
            on_sel2(selected);
        });

        stack.add_named(&list_box, Some("list"));
        stack.add_named(&flow_box, Some("grid"));
        // Deseleccionar al hacer clic en cualquier espacio vacío o fondo
        let deselect_empty = {
            let lb_unsel = list_box.clone();
            let fb_unsel = flow_box.clone();
            move |widget: &gtk4::Widget, x: f64, y: f64| {
                if let Some(target) = widget.pick(x, y, gtk4::PickFlags::DEFAULT) {
                    let is_on_row = target.type_() == gtk4::ListBoxRow::static_type()
                        || target.ancestor(gtk4::ListBoxRow::static_type()).is_some();
                    let is_on_card = target.type_() == gtk4::FlowBoxChild::static_type()
                        || target.ancestor(gtk4::FlowBoxChild::static_type()).is_some();
                    let is_on_scrollbar = target.type_() == gtk4::Scrollbar::static_type()
                        || target.ancestor(gtk4::Scrollbar::static_type()).is_some();
                    if !is_on_row && !is_on_card && !is_on_scrollbar {
                        lb_unsel.unselect_all();
                        fb_unsel.unselect_all();
                    }
                } else {
                    lb_unsel.unselect_all();
                    fb_unsel.unselect_all();
                }
            }
        };

        let click_gesture = gtk4::GestureClick::new();
        click_gesture.set_button(gdk::BUTTON_PRIMARY);
        let de1 = deselect_empty.clone();
        click_gesture.connect_pressed(move |gesture, _n_press, x, y| {
            if let Some(widget) = gesture.widget() {
                de1(&widget, x, y);
            }
        });
        container.add_controller(click_gesture);

        let lb_gesture = gtk4::GestureClick::new();
        lb_gesture.set_button(gdk::BUTTON_PRIMARY);
        let de2 = deselect_empty.clone();
        lb_gesture.connect_pressed(move |gesture, _n_press, x, y| {
            if let Some(widget) = gesture.widget() {
                de2(&widget, x, y);
            }
        });
        list_box.add_controller(lb_gesture);

        let fb_gesture = gtk4::GestureClick::new();
        fb_gesture.set_button(gdk::BUTTON_PRIMARY);
        let de3 = deselect_empty;
        fb_gesture.connect_pressed(move |gesture, _n_press, x, y| {
            if let Some(widget) = gesture.widget() {
                de3(&widget, x, y);
            }
        });
        flow_box.add_controller(fb_gesture);

        let on_context_menu: Rc<RefCell<Option<Rc<dyn Fn(bool, f64, f64)>>>> = Rc::new(RefCell::new(None));

        // Menú contextual en clic derecho (Secundario)
        let right_click = gtk4::GestureClick::new();
        right_click.set_button(gdk::BUTTON_SECONDARY);
        let lb_rc = list_box.clone();
        let fb_rc = flow_box.clone();
        let cb_rc = on_context_menu.clone();

        right_click.connect_pressed(move |gesture, _, x, y| {
            if let Some(widget) = gesture.widget() {
                if let Some(target) = widget.pick(x, y, gtk4::PickFlags::DEFAULT) {
                    let row_opt = if target.type_() == gtk4::ListBoxRow::static_type() {
                        target.clone().downcast::<gtk4::ListBoxRow>().ok()
                    } else {
                        target.ancestor(gtk4::ListBoxRow::static_type()).and_then(|w| w.downcast::<gtk4::ListBoxRow>().ok())
                    };

                    let card_opt = if target.type_() == gtk4::FlowBoxChild::static_type() {
                        target.clone().downcast::<gtk4::FlowBoxChild>().ok()
                    } else {
                        target.ancestor(gtk4::FlowBoxChild::static_type()).and_then(|w| w.downcast::<gtk4::FlowBoxChild>().ok())
                    };

                    if let Some(row) = row_opt {
                        let is_selected = lb_rc.selected_rows().iter().any(|r| r == &row);
                        if !is_selected {
                            lb_rc.unselect_all();
                            lb_rc.select_row(Some(&row));
                            row.grab_focus();
                        }
                        if let Some(ref cb) = *cb_rc.borrow() {
                            cb(true, x, y);
                        }
                    } else if let Some(child) = card_opt {
                        let is_selected = fb_rc.selected_children().iter().any(|c| c == &child);
                        if !is_selected {
                            fb_rc.unselect_all();
                            fb_rc.select_child(&child);
                            child.grab_focus();
                        }
                        if let Some(ref cb) = *cb_rc.borrow() {
                            cb(true, x, y);
                        }
                    } else {
                        lb_rc.unselect_all();
                        fb_rc.unselect_all();
                        if let Some(ref cb) = *cb_rc.borrow() {
                            cb(false, x, y);
                        }
                    }
                } else {
                    lb_rc.unselect_all();
                    fb_rc.unselect_all();
                    if let Some(ref cb) = *cb_rc.borrow() {
                        cb(false, x, y);
                    }
                }
            }
        });
        container.add_controller(right_click);

        let (thumbnail_sender, thumbnail_receiver) = channel::<(PathBuf, Vec<u8>)>();
        let thumbnail_map: Rc<RefCell<HashMap<PathBuf, glib::WeakRef<gtk4::Image>>>> = Rc::new(RefCell::new(HashMap::new()));
        let texture_cache: Rc<RefCell<HashMap<PathBuf, gdk::Texture>>> = Rc::new(RefCell::new(HashMap::new()));

        let map_recv = thumbnail_map.clone();
        let cache_recv = texture_cache.clone();
        glib::timeout_add_local(Duration::from_millis(16), move || {
            let mut processed = 0;
            while let Ok((path, data_bytes)) = thumbnail_receiver.try_recv() {
                let bytes = glib::Bytes::from_owned(data_bytes);
                if let Ok(texture) = gdk::Texture::from_bytes(&bytes) {
                    {
                        let mut cache = cache_recv.borrow_mut();
                        if cache.len() > 600 {
                            cache.clear();
                        }
                        cache.insert(path.clone(), texture.clone());
                    }
                    if let Some(weak) = map_recv.borrow().get(&path) {
                        if let Some(img) = weak.upgrade() {
                            img.set_paintable(Some(&texture));
                        }
                    }
                }
                processed += 1;
                if processed >= 40 {
                    break;
                }
            }
            glib::ControlFlow::Continue
        });

        stack.set_visible_child_name(if is_grid { "grid" } else { "list" });
        container.set_child(Some(&stack));

        Self {
            container,
            stack,
            list_box,
            flow_box,
            view_mode,
            zoom_level,
            entries,
            visible_indices,
            filter_query,
            thumbnail_sender,
            thumbnail_map,
            texture_cache,
            on_context_menu,
        }
    }

    pub fn connect_context_menu<F>(&self, callback: F)
    where
        F: Fn(bool, f64, f64) + 'static,
    {
        *self.on_context_menu.borrow_mut() = Some(Rc::new(callback));
    }

    pub fn connect_open<F>(&self, on_open: F)
    where
        F: Fn(FileEntry) + 'static + Clone,
    {
        let entries = self.entries.clone();
        let on_open_list = on_open.clone();
        self.list_box.connect_row_activated(move |_, row| {
            let entry_opt = if let Ok(idx) = row.widget_name().parse::<usize>() {
                let ents = entries.borrow();
                ents.get(idx).cloned()
            } else {
                None
            };
            if let Some(entry) = entry_opt {
                let cb = on_open_list.clone();
                glib::idle_add_local_once(move || {
                    cb(entry);
                });
            }
        });

        let entries2 = self.entries.clone();
        self.flow_box.connect_child_activated(move |_, child| {
            let entry_opt = if let Ok(idx) = child.widget_name().parse::<usize>() {
                let ents = entries2.borrow();
                ents.get(idx).cloned()
            } else {
                None
            };
            if let Some(entry) = entry_opt {
                let cb = on_open.clone();
                glib::idle_add_local_once(move || {
                    cb(entry);
                });
            }
        });
    }

    pub fn widget(&self) -> &ScrolledWindow {
        &self.container
    }

    pub fn is_grid(&self) -> bool {
        *self.view_mode.borrow() == ViewMode::Grid
    }

    #[allow(dead_code)]
    pub fn view_mode(&self) -> ViewMode {
        *self.view_mode.borrow()
    }

    pub fn set_view_mode(&self, mode: ViewMode) {
        *self.view_mode.borrow_mut() = mode;
        self.stack.set_visible_child_name(if mode == ViewMode::Grid { "grid" } else { "list" });
        self.rebuild_list();
        self.grab_focus();
    }

    pub fn toggle_view_mode(&self) -> ViewMode {
        let new_mode = if self.is_grid() { ViewMode::List } else { ViewMode::Grid };
        self.set_view_mode(new_mode);
        new_mode
    }

    pub fn current_zoom(&self) -> i32 {
        *self.zoom_level.borrow()
    }

    pub fn zoom_percent(&self) -> u32 {
        match *self.zoom_level.borrow() {
            -2 => 60,
            -1 => 80,
            0 => 100,
            1 => 125,
            2 => 150,
            3 => 175,
            _ => 200,
        }
    }

    pub fn zoom_in(&self) -> i32 {
        let cur = *self.zoom_level.borrow();
        if cur < 4 {
            *self.zoom_level.borrow_mut() = cur + 1;
            self.rebuild_list();
        }
        *self.zoom_level.borrow()
    }

    pub fn zoom_out(&self) -> i32 {
        let cur = *self.zoom_level.borrow();
        if cur > -2 {
            *self.zoom_level.borrow_mut() = cur - 1;
            self.rebuild_list();
        }
        *self.zoom_level.borrow()
    }

    pub fn zoom_reset(&self) -> i32 {
        *self.zoom_level.borrow_mut() = 0;
        self.rebuild_list();
        0
    }

    pub fn grab_focus(&self) {
        if self.is_grid() {
            if let Some(child) = self.flow_box.selected_children().into_iter().next() {
                child.grab_focus();
            } else if let Some(first) = self.flow_box.child_at_index(0) {
                self.flow_box.select_child(&first);
                first.grab_focus();
            } else {
                self.flow_box.grab_focus();
            }
        } else {
            if let Some(row) = self.list_box.selected_row() {
                row.grab_focus();
            } else if let Some(first) = self.list_box.row_at_index(0) {
                self.list_box.select_row(Some(&first));
                first.grab_focus();
            } else {
                self.list_box.grab_focus();
            }
        }
    }

    pub fn set_entries(&self, new_entries: Vec<FileEntry>) {
        *self.entries.borrow_mut() = new_entries;
        self.rebuild_list();
    }

    pub fn set_filter(&self, query: &str) {
        *self.filter_query.borrow_mut() = query.to_lowercase();
        self.rebuild_list();
    }

    pub fn selected_entry(&self) -> Option<FileEntry> {
        self.selected_entries().into_iter().next()
    }

    pub fn selected_entries(&self) -> Vec<FileEntry> {
        let ents = self.entries.borrow();
        if self.is_grid() {
            self.flow_box
                .selected_children()
                .into_iter()
                .filter_map(|child| child.widget_name().parse::<usize>().ok())
                .filter_map(|idx| ents.get(idx).cloned())
                .collect()
        } else {
            self.list_box
                .selected_rows()
                .into_iter()
                .filter_map(|row| row.widget_name().parse::<usize>().ok())
                .filter_map(|idx| ents.get(idx).cloned())
                .collect()
        }
    }

    pub fn select_all(&self) {
        if self.is_grid() {
            self.flow_box.select_all();
        } else {
            self.list_box.select_all();
        }
    }

    pub fn unselect_all(&self) {
        if self.is_grid() {
            self.flow_box.unselect_all();
        } else {
            self.list_box.unselect_all();
        }
    }

    pub fn connect_drop_files<F>(&self, on_drop: F)
    where
        F: Fn(Vec<PathBuf>) + 'static + Clone,
    {
        let drop_target = DropTarget::new(
            gdk::FileList::static_type(),
            gdk::DragAction::all(),
        );
        let on_drop_cb = on_drop;
        drop_target.connect_drop(move |_, val, _, _| {
            if let Ok(file_list) = val.get::<gdk::FileList>() {
                let paths: Vec<PathBuf> = file_list
                    .files()
                    .into_iter()
                    .filter_map(|f| f.path())
                    .collect();
                if !paths.is_empty() {
                    on_drop_cb(paths);
                    return true;
                }
            }
            false
        });
        self.container.add_controller(drop_target);
    }

    pub fn select_next(&self) {
        let count = self.visible_indices.borrow().len();
        if count == 0 {
            return;
        }

        if self.is_grid() {
            let cur = self.flow_box.selected_children().first().map(|c| c.index());
            if let Some(cur) = cur {
                if cur + 1 < count as i32 {
                    if let Some(next) = self.flow_box.child_at_index(cur + 1) {
                        self.flow_box.unselect_all();
                        self.flow_box.select_child(&next);
                        next.grab_focus();
                        self.scroll_to_widget(&next);
                    }
                }
            } else if let Some(first) = self.flow_box.child_at_index(0) {
                self.flow_box.unselect_all();
                self.flow_box.select_child(&first);
                first.grab_focus();
                self.scroll_to_widget(&first);
            }
        } else {
            let cur = self.list_box.selected_rows().first().map(|r| r.index());
            if let Some(cur) = cur {
                if cur + 1 < count as i32 {
                    if let Some(next) = self.list_box.row_at_index(cur + 1) {
                        self.list_box.unselect_all();
                        self.list_box.select_row(Some(&next));
                        next.grab_focus();
                        self.scroll_to_widget(&next);
                    }
                }
            } else if let Some(first) = self.list_box.row_at_index(0) {
                self.list_box.unselect_all();
                self.list_box.select_row(Some(&first));
                first.grab_focus();
                self.scroll_to_widget(&first);
            }
        }
    }

    pub fn select_prev(&self) {
        let count = self.visible_indices.borrow().len();
        if count == 0 {
            return;
        }

        if self.is_grid() {
            let cur = self.flow_box.selected_children().first().map(|c| c.index());
            if let Some(cur) = cur {
                if cur > 0 {
                    if let Some(prev) = self.flow_box.child_at_index(cur - 1) {
                        self.flow_box.unselect_all();
                        self.flow_box.select_child(&prev);
                        prev.grab_focus();
                        self.scroll_to_widget(&prev);
                    }
                }
            } else if let Some(first) = self.flow_box.child_at_index(0) {
                self.flow_box.unselect_all();
                self.flow_box.select_child(&first);
                first.grab_focus();
                self.scroll_to_widget(&first);
            }
        } else {
            let cur = self.list_box.selected_rows().first().map(|r| r.index());
            if let Some(cur) = cur {
                if cur > 0 {
                    if let Some(prev) = self.list_box.row_at_index(cur - 1) {
                        self.list_box.unselect_all();
                        self.list_box.select_row(Some(&prev));
                        prev.grab_focus();
                        self.scroll_to_widget(&prev);
                    }
                }
            } else if let Some(first) = self.list_box.row_at_index(0) {
                self.list_box.unselect_all();
                self.list_box.select_row(Some(&first));
                first.grab_focus();
                self.scroll_to_widget(&first);
            }
        }
    }

    pub fn select_first(&self) {
        if self.is_grid() {
            if let Some(first) = self.flow_box.child_at_index(0) {
                self.flow_box.unselect_all();
                self.flow_box.select_child(&first);
                first.grab_focus();
                self.scroll_to_widget(&first);
            }
        } else {
            if let Some(first) = self.list_box.row_at_index(0) {
                self.list_box.unselect_all();
                self.list_box.select_row(Some(&first));
                first.grab_focus();
                self.scroll_to_widget(&first);
            }
        }
    }

    pub fn select_last(&self) {
        let count = self.visible_indices.borrow().len();
        if count > 0 {
            if self.is_grid() {
                if let Some(last) = self.flow_box.child_at_index(count as i32 - 1) {
                    self.flow_box.unselect_all();
                    self.flow_box.select_child(&last);
                    last.grab_focus();
                    self.scroll_to_widget(&last);
                }
            } else {
                if let Some(last) = self.list_box.row_at_index(count as i32 - 1) {
                    self.list_box.unselect_all();
                    self.list_box.select_row(Some(&last));
                    last.grab_focus();
                    self.scroll_to_widget(&last);
                }
            }
        }
    }

    pub fn select_name(&self, name: &str) {
        let target_row_idx = {
            let ents = self.entries.borrow();
            let vis = self.visible_indices.borrow();
            vis.iter().position(|&orig_idx| {
                ents.get(orig_idx).map(|e| e.name == name).unwrap_or(false)
            })
        };

        if let Some(row_idx) = target_row_idx {
            if self.is_grid() {
                if let Some(child) = self.flow_box.child_at_index(row_idx as i32) {
                    self.flow_box.unselect_all();
                    self.flow_box.select_child(&child);
                    child.grab_focus();
                    self.scroll_to_widget(&child);
                }
            } else {
                if let Some(row) = self.list_box.row_at_index(row_idx as i32) {
                    self.list_box.unselect_all();
                    self.list_box.select_row(Some(&row));
                    row.grab_focus();
                    self.scroll_to_widget(&row);
                }
            }
        }
    }

    fn scroll_to_widget<W: IsA<gtk4::Widget>>(&self, widget: &W) {
        let adj = self.container.vadjustment();
        let alloc = widget.allocation();
        let y = alloc.y() as f64;
        let height = alloc.height() as f64;
        let cur_val = adj.value();
        let page_size = adj.page_size();

        if y < cur_val {
            adj.set_value(y);
        } else if y + height > cur_val + page_size {
            adj.set_value(y + height - page_size);
        }
    }

    fn rebuild_list(&self) {
        let is_grid = self.is_grid();
        let zoom = *self.zoom_level.borrow();

        // Limpiar widgets existentes
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
        while let Some(child) = self.flow_box.first_child() {
            self.flow_box.remove(&child);
        }
        self.thumbnail_map.borrow_mut().clear();

        let (list_rows, grid_children, vis) = {
            let ents = self.entries.borrow();
            let query = self.filter_query.borrow();
            let mut vis = Vec::new();
            let mut l_rows = Vec::new();
            let mut g_children = Vec::new();

            for (idx, entry) in ents.iter().enumerate() {
                if query.is_empty() || entry.name.to_lowercase().contains(&*query) {
                    vis.push(idx);
                    if is_grid {
                        g_children.push(self.create_grid_child(idx, entry, zoom));
                    } else {
                        l_rows.push(self.create_list_row(idx, entry, zoom));
                    }
                }
            }
            (l_rows, g_children, vis)
        };

        *self.visible_indices.borrow_mut() = vis;

        if is_grid {
            for child in grid_children {
                self.flow_box.insert(&child, -1);
            }
            if let Some(first) = self.flow_box.child_at_index(0) {
                self.flow_box.select_child(&first);
            }
        } else {
            for row in list_rows {
                self.list_box.append(&row);
            }
            if let Some(first) = self.list_box.row_at_index(0) {
                self.list_box.select_row(Some(&first));
            }
        }
    }

    fn create_list_row(&self, index: usize, entry: &FileEntry, zoom: i32) -> ListBoxRow {
        let row = ListBoxRow::new();
        row.set_widget_name(&format!("{index}"));
        row.set_focusable(true);

        let icon_size = match zoom {
            -2 => 12,
            -1 => 14,
            0 => 16,
            1 => 20,
            2 => 24,
            3 => 28,
            _ => 32,
        };

        let row_padding = match zoom {
            -2 => 1,
            -1 => 2,
            0 => 3,
            1 => 4,
            2 => 6,
            3 => 8,
            _ => 10,
        };

        let row_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .margin_top(row_padding)
            .margin_bottom(row_padding)
            .margin_start(6)
            .margin_end(6)
            .build();

        let is_media = !entry.is_dir && is_media_file(&entry.name);

        let icon_widget = gtk4::Image::builder()
            .icon_name(entry.icon_name)
            .pixel_size(icon_size)
            .build();

        if is_media {
            let path = entry.path.clone();
            let target_size = icon_size;
            if let Some(cached_texture) = self.texture_cache.borrow().get(&path) {
                icon_widget.set_paintable(Some(cached_texture));
            } else {
                self.thumbnail_map.borrow_mut().insert(path.clone(), icon_widget.downgrade());
                let sender = self.thumbnail_sender.clone();
                let _ = thumbnail_pool().push(move || {
                    if let Some(bytes) = load_media_thumbnail(&path, target_size) {
                        let _ = sender.send((path, bytes));
                    }
                });
            }
        }

        // Nombre del archivo
        let mut name_classes = vec!["file-name"];
        if entry.is_dir {
            name_classes.push("folder-name");
            name_classes.push("heading");
        }
        if entry.is_hidden {
            name_classes.push("dim-label");
        }

        let name_label = Label::builder()
            .label(&entry.name)
            .halign(Align::Start)
            .hexpand(true)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .css_classes(name_classes)
            .build();

        // Permisos UNIX
        let perm_label = Label::builder()
            .label(&entry.permissions)
            .halign(Align::End)
            .css_classes(["dim-label", "caption", "monospace"])
            .build();

        // Tamaño
        let size_label = Label::builder()
            .label(&entry.formatted_size)
            .halign(Align::End)
            .width_chars(11)
            .css_classes(["caption", "numeric", "file-size"])
            .build();

        // Fecha de modificación
        let date_label = Label::builder()
            .label(&entry.formatted_date)
            .halign(Align::End)
            .width_chars(16)
            .css_classes(["dim-label", "caption", "numeric"])
            .build();

        row_box.append(&icon_widget);
        row_box.append(&name_label);
        row_box.append(&perm_label);
        row_box.append(&size_label);
        row_box.append(&date_label);

        row.set_child(Some(&row_box));

        // Drag and Drop (Arrastrar archivos)
        let drag_source = DragSource::new();
        drag_source.set_actions(gdk::DragAction::all());
        let entries_drag = self.entries.clone();
        let list_box_drag = self.list_box.clone();
        let row_idx = index;
        drag_source.connect_prepare(move |_, _, _| {
            let ents = entries_drag.borrow();
            let selected_indices: Vec<usize> = list_box_drag
                .selected_rows()
                .into_iter()
                .filter_map(|r| r.widget_name().parse::<usize>().ok())
                .collect();

            let paths: Vec<PathBuf> = if selected_indices.contains(&row_idx) && selected_indices.len() > 1 {
                selected_indices
                    .iter()
                    .filter_map(|&i| ents.get(i).map(|e| e.path.clone()))
                    .collect()
            } else if let Some(e) = ents.get(row_idx) {
                vec![e.path.clone()]
            } else {
                return None;
            };

            let gfiles: Vec<gio::File> = paths.iter().map(|p| gio::File::for_path(p)).collect();
            let file_list = gdk::FileList::from_array(&gfiles);
            Some(gdk::ContentProvider::for_value(&file_list.to_value()))
        });
        row.add_controller(drag_source);

        row
    }

    fn create_grid_child(&self, index: usize, entry: &FileEntry, zoom: i32) -> FlowBoxChild {
        let child = FlowBoxChild::new();
        child.set_widget_name(&format!("{index}"));
        child.set_focusable(true);

        let icon_size = match zoom {
            -2 => 24,
            -1 => 30,
            0 => 36,
            1 => 48,
            2 => 64,
            3 => 80,
            _ => 96,
        };

        let card_width = match zoom {
            -2 => 68,
            -1 => 78,
            0 => 88,
            1 => 110,
            2 => 135,
            3 => 160,
            _ => 190,
        };

        let max_chars = match zoom {
            -2 => 8,
            -1 => 10,
            0 => 11,
            1 => 14,
            2 => 16,
            3 => 18,
            _ => 22,
        };

        let card_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(2)
            .halign(Align::Center)
            .valign(Align::Center)
            .width_request(card_width)
            .margin_top(4)
            .margin_bottom(4)
            .margin_start(2)
            .margin_end(2)
            .css_classes(["file-grid-card"])
            .build();

        let is_media = !entry.is_dir && is_media_file(&entry.name);

        let icon_widget = gtk4::Image::builder()
            .icon_name(entry.icon_name)
            .pixel_size(icon_size)
            .halign(Align::Center)
            .build();

        if is_media {
            let path = entry.path.clone();
            let target_size = icon_size;
            if let Some(cached_texture) = self.texture_cache.borrow().get(&path) {
                icon_widget.set_paintable(Some(cached_texture));
            } else {
                self.thumbnail_map.borrow_mut().insert(path.clone(), icon_widget.downgrade());
                let sender = self.thumbnail_sender.clone();
                let _ = thumbnail_pool().push(move || {
                    if let Some(bytes) = load_media_thumbnail(&path, target_size) {
                        let _ = sender.send((path, bytes));
                    }
                });
            }
        }

        let mut name_classes = vec!["file-name", "file-grid-name"];
        if entry.is_dir {
            name_classes.push("folder-name");
        }
        if entry.is_hidden {
            name_classes.push("dim-label");
        }

        let name_label = Label::builder()
            .label(&entry.name)
            .halign(Align::Center)
            .justify(gtk4::Justification::Center)
            .wrap(true)
            .wrap_mode(gtk4::pango::WrapMode::WordChar)
            .lines(2)
            .max_width_chars(max_chars)
            .ellipsize(gtk4::pango::EllipsizeMode::Middle)
            .css_classes(name_classes)
            .build();

        let size_label = Label::builder()
            .label(&entry.formatted_size)
            .halign(Align::Center)
            .css_classes(["dim-label", "caption", "file-grid-size"])
            .build();

        card_box.append(&icon_widget);
        card_box.append(&name_label);
        card_box.append(&size_label);

        child.set_child(Some(&card_box));

        // Drag and Drop (Arrastrar archivos)
        let drag_source = DragSource::new();
        drag_source.set_actions(gdk::DragAction::all());
        let entries_drag = self.entries.clone();
        let flow_box_drag = self.flow_box.clone();
        let child_idx = index;
        drag_source.connect_prepare(move |_, _, _| {
            let ents = entries_drag.borrow();
            let selected_indices: Vec<usize> = flow_box_drag
                .selected_children()
                .into_iter()
                .filter_map(|c| c.widget_name().parse::<usize>().ok())
                .collect();

            let paths: Vec<PathBuf> = if selected_indices.contains(&child_idx) && selected_indices.len() > 1 {
                selected_indices
                    .iter()
                    .filter_map(|&i| ents.get(i).map(|e| e.path.clone()))
                    .collect()
            } else if let Some(e) = ents.get(child_idx) {
                vec![e.path.clone()]
            } else {
                return None;
            };

            let gfiles: Vec<gio::File> = paths.iter().map(|p| gio::File::for_path(p)).collect();
            let file_list = gdk::FileList::from_array(&gfiles);
            Some(gdk::ContentProvider::for_value(&file_list.to_value()))
        });
        child.add_controller(drag_source);

        child
    }
}
