use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use libadwaita as adw;
use libadwaita::prelude::*;
use gtk4::{
    Align, Box, Button, Entry, EventControllerKey, EventControllerMotion, GestureClick, HeaderBar, Label, Orientation, Paned,
    Revealer, RevealerTransitionType, SearchBar, SearchEntry, Stack, ToggleButton,
};
use gtk4::gdk::{Key, ModifierType};

use crate::fs::entry::FileEntry;
use crate::fs::operations;
use crate::ui::context_menu::{ContextMenu, ContextMenuActions};
use crate::ui::command_palette::{show_command_palette, PaletteAction};
use crate::ui::settings::{show_settings_dialog, AppSettings};
use crate::ui::dialogs::{confirm_delete, prompt_input};
use crate::ui::file_list::{FileList, ViewMode};
use crate::ui::preview::PreviewPane;
use crate::ui::sidebar::Sidebar;
use crate::ui::keybindings::{ActionId, ShortcutsManager};
use crate::ui::tabs_bubble::{TabInfo, TabsBubble};

#[derive(Clone, Debug)]
pub struct TabState {
    pub path: PathBuf,
    pub history_back: Vec<PathBuf>,
    pub history_forward: Vec<PathBuf>,
}

pub struct MainWindow {
    window: adw::ApplicationWindow,
    toast_overlay: adw::ToastOverlay,
    current_path: Rc<RefCell<PathBuf>>,
    history_back: Rc<RefCell<Vec<PathBuf>>>,
    history_forward: Rc<RefCell<Vec<PathBuf>>>,
    tabs: Rc<RefCell<Vec<TabState>>>,
    active_tab: Rc<RefCell<usize>>,
    tabs_bubble: TabsBubble,
    shortcuts: Rc<RefCell<ShortcutsManager>>,
    show_hidden: Rc<RefCell<bool>>,
    clipboard: Rc<RefCell<Option<(Vec<PathBuf>, bool)>>>, // (paths, is_cut)
    file_list: FileList,
    preview_pane: PreviewPane,
    sidebar: Sidebar,
    search_bar: SearchBar,
    search_entry: SearchEntry,
    path_stack: Stack,
    path_btn: Button,
    path_entry: Entry,
    status_label: Label,
    btn_back: Button,
    btn_forward: Button,
    header: HeaderBar,
    header_revealer: Revealer,
    settings: Rc<RefCell<AppSettings>>,
    btn_sidebar: Button,
    btn_view_mode: Button,
    btn_keyboard: Button,
    btn_settings: Button,
    context_menu: ContextMenu,
    paned_main: Paned,
}

impl MainWindow {
    pub fn new(app: &adw::Application) -> Rc<Self> {
        let initial_path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Explor")
            .default_width(1080)
            .default_height(680)
            .build();

        let toast_overlay = adw::ToastOverlay::new();

        let current_path = Rc::new(RefCell::new(initial_path.clone()));
        let history_back = Rc::new(RefCell::new(Vec::new()));
        let history_forward = Rc::new(RefCell::new(Vec::new()));
        let show_hidden = Rc::new(RefCell::new(false));
        let clipboard: Rc<RefCell<Option<(Vec<PathBuf>, bool)>>> = Rc::new(RefCell::new(None));

        let settings = Rc::new(RefCell::new(AppSettings::load()));
        let is_auto_hide = settings.borrow().auto_hide_navbar;
        let is_show_hidden = settings.borrow().show_hidden;
        let is_show_preview = settings.borrow().show_preview;

        *show_hidden.borrow_mut() = is_show_hidden;
        crate::ui::theme::apply_icon_theme(&settings.borrow().icon_theme);

        let preview_pane = PreviewPane::new();
        preview_pane.widget().set_visible(is_show_preview);
        let tabs_bubble = TabsBubble::new();

        let initial_tab = TabState {
            path: initial_path.clone(),
            history_back: Vec::new(),
            history_forward: Vec::new(),
        };
        let tabs = Rc::new(RefCell::new(vec![initial_tab]));
        let active_tab = Rc::new(RefCell::new(0));
        let shortcuts = Rc::new(RefCell::new(ShortcutsManager::load()));

        let preview_clone = preview_pane.clone();
        let on_selection = move |selected: Vec<FileEntry>| {
            let count = selected.len();
            if count == 1 {
                preview_clone.update(&selected[0]);
            } else if count > 1 {
                preview_clone.update_multi(&selected);
            } else {
                preview_clone.clear();
            }
        };

        let file_list = FileList::new(on_selection, settings.borrow().is_grid_view, settings.borrow().zoom_level);
        let sidebar = Sidebar::new();

        // Headerbar
        let header = HeaderBar::new();
        if settings.borrow().compact_navbar {
            header.add_css_class("compact-navbar");
        }

        // Botones de navegación historial
        let nav_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(4)
            .build();

        let btn_sidebar = Button::builder()
            .icon_name("sidebar-show-symbolic")
            .tooltip_text("Colapsar / Expandir barra lateral (Ctrl+B)")
            .build();

        let btn_back = Button::builder()
            .icon_name("go-previous-symbolic")
            .tooltip_text("Back (Alt+Left)")
            .sensitive(false)
            .build();

        let btn_forward = Button::builder()
            .icon_name("go-next-symbolic")
            .tooltip_text("Forward (Alt+Right)")
            .sensitive(false)
            .build();

        let btn_up = Button::builder()
            .icon_name("go-up-symbolic")
            .tooltip_text("Parent directory (Backspace / Alt+Up)")
            .build();

        nav_box.append(&btn_sidebar);
        nav_box.append(&btn_back);
        nav_box.append(&btn_forward);
        nav_box.append(&btn_up);
        header.pack_start(&nav_box);

        // Barra de ruta compacta (Stack entre botón pill con copy y entry editable)
        let path_stack = Stack::new();
        let path_pill_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(4)
            .css_classes(["path-pill"])
            .halign(Align::Center)
            .build();

        let path_btn = Button::builder()
            .label(&initial_path.display().to_string())
            .css_classes(["flat", "path-btn-inner"])
            .tooltip_text("Click or press Ctrl+L to edit path")
            .build();

        let btn_copy_path = Button::builder()
            .icon_name("edit-copy-symbolic")
            .css_classes(["flat", "circular", "path-copy-btn"])
            .tooltip_text("Copy current path (Ctrl+Shift+C)")
            .build();

        path_pill_box.append(&path_btn);
        path_pill_box.append(&btn_copy_path);

        let path_entry = Entry::builder()
            .text(&initial_path.display().to_string())
            .max_width_chars(36)
            .width_request(280)
            .halign(Align::Center)
            .secondary_icon_name("edit-copy-symbolic")
            .secondary_icon_tooltip_text("Copy current path")
            .build();

        path_stack.add_named(&path_pill_box, Some("button"));
        path_stack.add_named(&path_entry, Some("entry"));
        path_stack.set_visible_child_name("button");
        path_stack.set_halign(Align::Center);
        header.set_title_widget(Some(&path_stack));

        // Botones derecha (limpio sin botón redundante de ocultos)
        let right_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(4)
            .build();

        let btn_search = ToggleButton::builder()
            .icon_name("system-search-symbolic")
            .tooltip_text("Search files (Ctrl+F)")
            .build();

        let btn_keyboard = Button::builder()
            .icon_name("input-keyboard-symbolic")
            .tooltip_text("Command palette and shortcuts (Ctrl+P)")
            .build();

        let is_grid_view = settings.borrow().is_grid_view;
        let btn_view_mode = Button::builder()
            .icon_name(if is_grid_view { "view-list-symbolic" } else { "view-grid-symbolic" })
            .tooltip_text(if is_grid_view { "Cambiar a vista de lista (v)" } else { "Cambiar a vista de cuadrícula (v)" })
            .build();

        let btn_settings = Button::builder()
            .icon_name("emblem-system-symbolic")
            .tooltip_text("Settings / Preferences (Ctrl+,)")
            .build();

        let status_label = Label::builder()
            .label("")
            .halign(Align::End)
            .css_classes(["caption", "dim-label", "numeric"])
            .margin_end(8)
            .build();

        right_box.append(&status_label);
        right_box.append(&btn_view_mode);
        right_box.append(&btn_search);
        right_box.append(&btn_keyboard);
        right_box.append(&btn_settings);
        header.pack_end(&right_box);

        // Barra de búsqueda
        let search_entry = SearchEntry::builder()
            .placeholder_text("Filter files in current directory...")
            .hexpand(true)
            .build();

        let search_bar = SearchBar::builder()
            .child(&search_entry)
            .key_capture_widget(&window)
            .build();

        btn_search.bind_property("active", &search_bar, "search-mode-enabled")
            .bidirectional()
            .build();

        // Centro: Tabs bubble en la parte superior si hay múltiples pestañas, y file_list llenando todo el espacio sin barra inferior
        let center_box = Box::builder()
            .orientation(Orientation::Vertical)
            .hexpand(true)
            .vexpand(true)
            .build();

        center_box.append(tabs_bubble.widget());
        center_box.append(file_list.widget());

        // Menú contextual para clic derecho en archivos y espacios vacíos
        let context_menu = ContextMenu::new(file_list.widget());

        // Paneles principales
        let paned_right = Paned::builder()
            .orientation(Orientation::Horizontal)
            .start_child(&center_box)
            .end_child(preview_pane.widget())
            .position(720)
            .shrink_start_child(false)
            .shrink_end_child(false)
            .build();

        // La barra lateral de accesos directos usa solo el ancho necesario (135px) y lo mantiene fijo
        let is_collapsed = settings.borrow().sidebar_collapsed;
        if is_collapsed {
            sidebar.set_collapsed(true);
        }
        let initial_sidebar_pos = if is_collapsed { 48 } else { 135 };

        let paned_main = Paned::builder()
            .orientation(Orientation::Horizontal)
            .start_child(sidebar.widget())
            .end_child(&paned_right)
            .position(initial_sidebar_pos)
            .shrink_start_child(false)
            .shrink_end_child(false)
            .resize_start_child(false)
            .resize_end_child(true)
            .build();

        // Revealer para auto-ocultar la barra de navegación superior
        let header_revealer = Revealer::builder()
            .child(&header)
            .transition_type(RevealerTransitionType::SlideDown)
            .transition_duration(180)
            .reveal_child(!is_auto_hide)
            .build();

        // Sensor superior invisible para desplegar la barra con el ratón
        let hover_sensor = Box::builder()
            .orientation(Orientation::Horizontal)
            .height_request(4)
            .hexpand(true)
            .build();

        let motion_sensor = EventControllerMotion::new();
        let rev_clone = header_revealer.clone();
        motion_sensor.connect_enter(move |_, _, _| {
            rev_clone.set_reveal_child(true);
        });
        hover_sensor.add_controller(motion_sensor);

        let motion_header = EventControllerMotion::new();
        let rev_clone2 = header_revealer.clone();
        let settings_clone = settings.clone();
        motion_header.connect_leave(move |_| {
            if settings_clone.borrow().auto_hide_navbar {
                rev_clone2.set_reveal_child(false);
            }
        });
        header_revealer.add_controller(motion_header);

        // Contenedor vertical principal
        let main_content = Box::builder()
            .orientation(Orientation::Vertical)
            .build();

        main_content.append(&hover_sensor);
        main_content.append(&header_revealer);
        main_content.append(&search_bar);
        main_content.append(&paned_main);

        toast_overlay.set_child(Some(&main_content));
        window.set_content(Some(&toast_overlay));

        let instance = Rc::new(Self {
            window,
            toast_overlay,
            current_path,
            history_back,
            history_forward,
            tabs,
            active_tab,
            tabs_bubble,
            shortcuts,
            show_hidden,
            clipboard,
            file_list,
            preview_pane,
            sidebar,
            search_bar,
            search_entry,
            path_stack,
            path_btn,
            path_entry,
            status_label,
            btn_back,
            btn_forward,
            header,
            header_revealer,
            settings,
            btn_sidebar,
            btn_view_mode,
            btn_keyboard,
            btn_settings,
            context_menu,
            paned_main,
        });

        instance.init_signals(&btn_up, &btn_copy_path);
        instance.navigate_to(initial_path, false);
        instance.update_tabs_ui();

        instance
    }

    pub fn open_entry(self: &Rc<Self>, entry: &FileEntry) {
        let is_dir = entry.is_dir || entry.path.is_dir();
        if is_dir {
            self.navigate_to(entry.path.clone(), true);
        } else if let Err(e) = operations::open_with_default(&entry.path) {
            self.show_toast(&e);
        }
    }

    pub fn update_action_bubble(&self) {
        // No-op: la barra inferior fue reemplazada por el menú contextual de clic derecho
    }

    pub fn toggle_sidebar_collapse(self: &Rc<Self>) {
        let is_collapsed = self.sidebar.toggle_collapse();
        if is_collapsed {
            self.paned_main.set_position(48);
            self.btn_sidebar.set_tooltip_text(Some("Expandir barra lateral (Ctrl+B)"));
            self.show_toast("Sidebar: Icons only");
        } else {
            self.paned_main.set_position(135);
            self.btn_sidebar.set_tooltip_text(Some("Colapsar barra lateral (Ctrl+B)"));
            self.show_toast("Sidebar: Compact");
        }
    }

    pub fn extract_selected(self: &Rc<Self>) {
        if let Some(entry) = self.file_list.selected_entry() {
            if entry.is_archive {
                let this = self.clone();
                let archive_path = entry.path.clone();
                let this_done = this.clone();

                let (sender, cancel_flag) = self.sidebar.start_operation(
                    "Extracting...",
                    "package-x-generic",
                    move |success, res_path| {
                        if success {
                            this_done.refresh();
                            if let Some(p) = res_path {
                                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                                    this_done.file_list.select_name(name);
                                }
                            }
                            this_done.update_action_bubble();
                        }
                    },
                );

                std::thread::spawn(move || {
                    operations::extract_with_progress(archive_path, sender, cancel_flag);
                });
            } else {
                self.show_toast("The selected item is not an archive");
            }
        }
    }

    pub fn compress_selected(self: &Rc<Self>) {
        let entries = self.file_list.selected_entries();
        if !entries.is_empty() {
            let this = self.clone();
            let paths: Vec<PathBuf> = entries.iter().map(|e| e.path.clone()).collect();
            let this_done = this.clone();

            let (sender, cancel_flag) = self.sidebar.start_operation(
                "Compressing...",
                "package-x-generic-symbolic",
                move |success, res_path| {
                    if success {
                        this_done.refresh();
                        if let Some(p) = res_path {
                            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                                this_done.file_list.select_name(name);
                            }
                        }
                        this_done.update_action_bubble();
                    }
                },
            );

            std::thread::spawn(move || {
                operations::compress_multiple_with_progress(paths, sender, cancel_flag);
            });
        } else {
            self.show_toast("Select one or more items to compress");
        }
    }

    pub fn present(&self) {
        self.window.present();
        self.file_list.grab_focus();
    }

    pub fn window(&self) -> &adw::ApplicationWindow {
        &self.window
    }

    pub fn open_target(&self, path_or_uri: &str, select_mode: bool) {
        let path = if path_or_uri.starts_with("file://") {
            if let Ok((p, _)) = glib::filename_from_uri(path_or_uri) {
                p
            } else {
                let stripped = &path_or_uri["file://".len()..];
                PathBuf::from(stripped)
            }
        } else if let Some(stripped) = path_or_uri.strip_prefix("~/") {
            dirs::home_dir().map(|h| h.join(stripped)).unwrap_or_else(|| PathBuf::from(path_or_uri))
        } else {
            let p = PathBuf::from(path_or_uri);
            if p.is_relative() {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")).join(p)
            } else {
                p
            }
        };

        let path = if let Ok(canonical) = path.canonicalize() {
            canonical
        } else {
            path
        };

        if select_mode || path.is_file() {
            let parent = path.parent().unwrap_or(&path).to_path_buf();
            let file_name = path.file_name().map(|n| n.to_string_lossy().to_string());
            self.navigate_to(parent, false);
            if let Some(name) = file_name {
                let fl = self.file_list.clone();
                fl.select_name(&name);
                let name_clone = name.clone();
                glib::timeout_add_local_once(std::time::Duration::from_millis(60), move || {
                    fl.select_name(&name_clone);
                });
            }
        } else if path.is_dir() {
            self.navigate_to(path, false);
        } else {
            if let Some(parent) = path.parent() {
                if parent.is_dir() {
                    self.navigate_to(parent.to_path_buf(), false);
                    if let Some(name) = path.file_name().map(|n| n.to_string_lossy().to_string()) {
                        let fl = self.file_list.clone();
                        fl.select_name(&name);
                    }
                    return;
                }
            }
            self.show_toast(&format!("No se encontró: {}", path.display()));
        }
    }

    fn init_signals(self: &Rc<Self>, btn_up: &Button, btn_copy_path: &Button) {
        // Conexión botón editar ruta
        let this = self.clone();
        self.path_btn.connect_clicked(move |_| {
            this.activate_path_entry();
        });

        // Conexión submit en path_entry
        let this = self.clone();
        self.path_entry.connect_activate(move |e| {
            let text = e.text().to_string();
            this.handle_path_input(&text);
        });

        // Conexión filtro search_entry
        let this = self.clone();
        self.search_entry.connect_search_changed(move |e| {
            this.file_list.set_filter(e.text().as_str());
        });

        // Botón atrás
        let this = self.clone();
        self.btn_back.connect_clicked(move |_| {
            this.go_back();
        });

        // Botón adelante
        let this = self.clone();
        self.btn_forward.connect_clicked(move |_| {
            this.go_forward();
        });

        // Botón subir directorio
        let this = self.clone();
        btn_up.connect_clicked(move |_| {
            this.go_parent();
        });

        // Conectar botón nueva pestaña (+) en barra de pestañas
        let this_tab2 = self.clone();
        self.tabs_bubble.connect_new_tab(move || {
            this_tab2.open_new_tab_for_selection();
        });

        let this_sel = self.clone();
        self.tabs_bubble.connect_tab_selected(move |idx| {
            this_sel.switch_to_tab(idx);
        });

        let this_close = self.clone();
        self.tabs_bubble.connect_tab_closed(move |idx| {
            this_close.close_tab(idx);
        });

        // Conectar botón de copiar ruta
        let this_copy = self.clone();
        btn_copy_path.connect_clicked(move |_| {
            this_copy.copy_current_path();
        });

        // Conectar icono secundario de copiar ruta en path_entry
        let this_copy2 = self.clone();
        self.path_entry.connect_icon_press(move |_, icon_pos| {
            if icon_pos == gtk4::EntryIconPosition::Secondary {
                this_copy2.copy_current_path();
            }
        });



        // Botón unificado de comandos y atajos (abre diálogo de personalización de atajos)
        let this_kb = self.clone();
        let sc_kb = self.shortcuts.clone();
        self.btn_keyboard.connect_clicked(move |_| {
            let this_clone = this_kb.clone();
            crate::ui::shortcuts_dialog::show_shortcuts_dialog(&this_kb.window, sc_kb.clone(), move || {
                this_clone.show_toast("Keyboard shortcuts updated");
            });
        });

        // Botón de configuración
        let this_cfg = self.clone();
        self.btn_settings.connect_clicked(move |_| {
            this_cfg.open_settings();
        });

        // Toggle barra lateral colapsada
        let this_sidebar = self.clone();
        self.btn_sidebar.connect_clicked(move |_| {
            this_sidebar.toggle_sidebar_collapse();
        });

        // Botón cambiar modo de vista (Lista / Cuadrícula)
        let this_vm = self.clone();
        self.btn_view_mode.connect_clicked(move |_| {
            this_vm.toggle_view_mode();
        });

        // Configurar acciones del menú contextual
        let this_open = self.clone();
        let this_tab = self.clone();
        let this_copy = self.clone();
        let this_cut = self.clone();
        let this_paste = self.clone();
        let this_rename = self.clone();
        let this_extract = self.clone();
        let this_compress = self.clone();
        let this_trash = self.clone();
        let this_perm_del = self.clone();
        let this_restore = self.clone();
        let this_empty_trash = self.clone();
        let this_new_file = self.clone();
        let this_new_folder = self.clone();
        let this_term = self.clone();
        let this_copy_path = self.clone();
        let this_sel_all = self.clone();
        let this_refresh = self.clone();

        self.context_menu.set_actions(ContextMenuActions {
            on_open: Rc::new(move || {
                if let Some(entry) = this_open.file_list.selected_entry() {
                    this_open.open_entry(&entry);
                }
            }),
            on_new_tab: Rc::new(move || {
                this_tab.open_new_tab_for_selection();
            }),
            on_copy: Rc::new(move || {
                this_copy.copy_selected(false);
            }),
            on_cut: Rc::new(move || {
                this_cut.copy_selected(true);
            }),
            on_paste: Rc::new(move || {
                this_paste.paste_clipboard();
            }),
            on_rename: Rc::new(move || {
                this_rename.rename_selected();
            }),
            on_extract: Rc::new(move || {
                this_extract.extract_selected();
            }),
            on_compress: Rc::new(move || {
                this_compress.compress_selected();
            }),
            on_trash: Rc::new(move || {
                this_trash.delete_selected();
            }),
            on_delete_permanently: Rc::new(move || {
                this_perm_del.delete_selected_permanently();
            }),
            on_restore_trash: Rc::new(move || {
                this_restore.restore_selected();
            }),
            on_empty_trash: Rc::new(move || {
                this_empty_trash.empty_trash();
            }),
            on_new_file: Rc::new(move || {
                this_new_file.new_file();
            }),
            on_new_folder: Rc::new(move || {
                this_new_folder.new_folder();
            }),
            on_terminal: Rc::new(move || {
                this_term.open_terminal();
            }),
            on_copy_path: Rc::new(move || {
                this_copy_path.copy_current_path();
            }),
            on_select_all: Rc::new(move || {
                this_sel_all.file_list.select_all();
            }),
            on_refresh: Rc::new(move || {
                this_refresh.refresh();
            }),
        });

        // Conectar menú contextual para clic derecho en archivos y espacio vacío
        let this_ctx = self.clone();
        self.file_list.connect_context_menu(move |is_item, x, y| {
            let has_clip = this_ctx.clipboard.borrow().is_some();
            let in_trash = operations::is_trash_path(&this_ctx.current_path.borrow());
            if is_item {
                let selected = this_ctx.file_list.selected_entries();
                this_ctx.context_menu.show_for_item(x, y, &selected, has_clip, in_trash);
            } else {
                this_ctx.context_menu.show_for_empty(x, y, has_clip, in_trash);
            }
        });

        // Conectar vaciar papelera desde la barra lateral
        let this_side_trash = self.clone();
        self.sidebar.connect_empty_trash(move || {
            this_side_trash.empty_trash();
        });

        // File list drop handler (Arrastrar y soltar archivos dentro de la carpeta actual)
        let this_drop = self.clone();
        self.file_list.connect_drop_files(move |paths: Vec<PathBuf>| {
            let current = this_drop.current_path.borrow().clone();
            
            // Si los archivos arrastrados se sueltan en el mismo directorio donde ya están,
            // se trata de una cancelación del arrastre y no debe crearse ninguna copia duplicada.
            let paths_to_copy: Vec<PathBuf> = paths
                .into_iter()
                .filter(|p| {
                    if let Some(parent) = p.parent() {
                        if parent == current {
                            return false;
                        }
                    }
                    true
                })
                .collect();

            if paths_to_copy.is_empty() {
                return;
            }

            let this_done = this_drop.clone();
            let count = paths_to_copy.len();
            let title = if count == 1 { "Copiando archivo..." } else { "Copiando archivos..." };
            let (sender, cancel_flag) = this_drop.sidebar.start_operation(
                title,
                "edit-copy-symbolic",
                move |success, res_path| {
                    if success {
                        this_done.refresh();
                        if let Some(p) = res_path {
                            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                                this_done.file_list.select_name(name);
                            }
                        }
                        this_done.update_action_bubble();
                    }
                },
            );

            std::thread::spawn(move || {
                operations::copy_multiple_with_progress(paths_to_copy, current, false, sender, cancel_flag);
            });
        });

        // File list open handler
        let this_open = self.clone();
        self.file_list.connect_open(move |entry: FileEntry| {
            this_open.open_entry(&entry);
        });

        // Sidebar click
        let this_side = self.clone();
        self.sidebar.connect_navigate(move |path: PathBuf| {
            this_side.navigate_to(path, true);
            this_side.file_list.grab_focus();
        });

        // Global Key Controller para atajos rápidos
        self.setup_keyboard_shortcuts();

        // Soporte para botones laterales del mouse (Navegación Atrás / Adelante)
        self.setup_mouse_navigation();
    }

    fn setup_keyboard_shortcuts(self: &Rc<Self>) {
        let key_controller = EventControllerKey::new();
        let this = self.clone();

        key_controller.connect_key_pressed(move |_, key, _, modifier| {
            let ctrl = modifier.contains(ModifierType::CONTROL_MASK);
            let alt = modifier.contains(ModifierType::ALT_MASK);
            let shift = modifier.contains(ModifierType::SHIFT_MASK);

            // Si se está escribiendo en el buscador o en la ruta:
            if this.search_entry.has_focus() {
                if key == Key::Escape {
                    this.search_bar.set_search_mode(false);
                    this.file_list.grab_focus();
                    return glib::Propagation::Stop;
                } else if key == Key::Down || key == Key::Return {
                    this.file_list.grab_focus();
                    return glib::Propagation::Stop;
                }
                return glib::Propagation::Proceed;
            }

            if this.path_entry.has_focus() {
                if key == Key::Escape {
                    this.path_stack.set_visible_child_name("button");
                    this.file_list.grab_focus();
                    return glib::Propagation::Stop;
                }
                return glib::Propagation::Proceed;
            }

            // Atajos directos de pestañas numéricas Ctrl+1..9 y Ctrl+A (Seleccionar todo)
            if ctrl && !alt && !shift {
                match key {
                    Key::_1 => { this.switch_to_tab(0); return glib::Propagation::Stop; }
                    Key::_2 => { this.switch_to_tab(1); return glib::Propagation::Stop; }
                    Key::_3 => { this.switch_to_tab(2); return glib::Propagation::Stop; }
                    Key::_4 => { this.switch_to_tab(3); return glib::Propagation::Stop; }
                    Key::_5 => { this.switch_to_tab(4); return glib::Propagation::Stop; }
                    Key::_6 => { this.switch_to_tab(5); return glib::Propagation::Stop; }
                    Key::_7 => { this.switch_to_tab(6); return glib::Propagation::Stop; }
                    Key::_8 => { this.switch_to_tab(7); return glib::Propagation::Stop; }
                    Key::_9 => { this.switch_to_tab(8); return glib::Propagation::Stop; }
                    Key::a | Key::A => { this.file_list.select_all(); return glib::Propagation::Stop; }
                    Key::c | Key::C => { this.copy_selected(false); return glib::Propagation::Stop; }
                    Key::x | Key::X => { this.copy_selected(true); return glib::Propagation::Stop; }
                    Key::v | Key::V => { this.paste_clipboard(); return glib::Propagation::Stop; }
                    Key::n | Key::N => { this.new_file(); return glib::Propagation::Stop; }
                    Key::r | Key::R => { this.refresh(); return glib::Propagation::Stop; }
                    _ => {}
                }
            }

            if ctrl && shift && !alt {
                match key {
                    Key::n | Key::N => { this.new_folder(); return glib::Propagation::Stop; }
                    Key::c | Key::C => { this.copy_current_path(); return glib::Propagation::Stop; }
                    _ => {}
                }
            }

            if !ctrl && !alt && !shift {
                match key {
                    Key::F2 => { this.rename_selected(); return glib::Propagation::Stop; }
                    Key::F5 => { this.refresh(); return glib::Propagation::Stop; }
                    Key::Delete | Key::KP_Delete => { this.delete_selected(); return glib::Propagation::Stop; }
                    Key::Menu => {
                        let selected = this.file_list.selected_entries();
                        let has_clip = this.clipboard.borrow().is_some();
                        let in_trash = operations::is_trash_path(&this.current_path.borrow());
                        if !selected.is_empty() {
                            this.context_menu.show_for_item(100.0, 100.0, &selected, has_clip, in_trash);
                        } else {
                            this.context_menu.show_for_empty(100.0, 100.0, has_clip, in_trash);
                        }
                        return glib::Propagation::Stop;
                    }
                    _ => {}
                }
            }

            if !ctrl && !alt && shift {
                if key == Key::Delete || key == Key::KP_Delete {
                    this.delete_selected_permanently();
                    return glib::Propagation::Stop;
                }
                if key == Key::F10 {
                    let selected = this.file_list.selected_entries();
                    let has_clip = this.clipboard.borrow().is_some();
                    let in_trash = operations::is_trash_path(&this.current_path.borrow());
                    if !selected.is_empty() {
                        this.context_menu.show_for_item(100.0, 100.0, &selected, has_clip, in_trash);
                    } else {
                        this.context_menu.show_for_empty(100.0, 100.0, has_clip, in_trash);
                    }
                    return glib::Propagation::Stop;
                }
            }

            // Comprobar si el evento coincide con una acción configurada (personalizada o por defecto)
            if let Some(action) = this.shortcuts.borrow().match_event(key, modifier) {
                match action {
                    ActionId::NewTab => { this.open_new_tab_for_selection(); return glib::Propagation::Stop; }
                    ActionId::CloseTab => { this.close_current_tab(); return glib::Propagation::Stop; }
                    ActionId::NextTab => { this.next_tab(); return glib::Propagation::Stop; }
                    ActionId::PrevTab => { this.prev_tab(); return glib::Propagation::Stop; }
                    ActionId::Copy => { this.copy_selected(false); return glib::Propagation::Stop; }
                    ActionId::Cut => { this.copy_selected(true); return glib::Propagation::Stop; }
                    ActionId::Paste => { this.paste_clipboard(); return glib::Propagation::Stop; }
                    ActionId::Extract => { this.extract_selected(); return glib::Propagation::Stop; }
                    ActionId::Compress => { this.compress_selected(); return glib::Propagation::Stop; }
                    ActionId::Trash => { this.delete_selected(); return glib::Propagation::Stop; }
                    ActionId::Rename => { this.rename_selected(); return glib::Propagation::Stop; }
                    ActionId::NewFile => { this.new_file(); return glib::Propagation::Stop; }
                    ActionId::NewFolder => { this.new_folder(); return glib::Propagation::Stop; }
                    ActionId::Terminal => { this.open_terminal(); return glib::Propagation::Stop; }
                    ActionId::CopyPath => { this.copy_current_path(); return glib::Propagation::Stop; }
                    ActionId::EditPath => { this.activate_path_entry(); return glib::Propagation::Stop; }
                    ActionId::ToggleViewMode => { this.toggle_view_mode(); return glib::Propagation::Stop; }
                    ActionId::ZoomIn => { this.zoom_in(); return glib::Propagation::Stop; }
                    ActionId::ZoomOut => { this.zoom_out(); return glib::Propagation::Stop; }
                    ActionId::ZoomReset => { this.zoom_reset(); return glib::Propagation::Stop; }
                    ActionId::TogglePreview => { this.toggle_preview(); return glib::Propagation::Stop; }
                    ActionId::ToggleSidebar => { this.toggle_sidebar_collapse(); return glib::Propagation::Stop; }
                    ActionId::ToggleHidden => { this.toggle_show_hidden(); return glib::Propagation::Stop; }
                    ActionId::ToggleNavbar => { this.toggle_navbar(); return glib::Propagation::Stop; }
                    ActionId::Search => {
                        this.search_bar.set_search_mode(true);
                        this.search_entry.grab_focus();
                        return glib::Propagation::Stop;
                    }
                    ActionId::CommandPalette => { this.open_command_palette(); return glib::Propagation::Stop; }
                    ActionId::OpenSettings => { this.open_settings(); return glib::Propagation::Stop; }
                    ActionId::ShowProgress => { this.sidebar.toggle_progress_popover(); return glib::Propagation::Stop; }
                    ActionId::SelectNext => { this.file_list.select_next(); return glib::Propagation::Stop; }
                    ActionId::SelectPrev => { this.file_list.select_prev(); return glib::Propagation::Stop; }
                    ActionId::OpenSelected => {
                        if let Some(entry) = this.file_list.selected_entry() {
                            this.open_entry(&entry);
                        }
                        return glib::Propagation::Stop;
                    }
                    ActionId::GoParent => { this.go_parent(); return glib::Propagation::Stop; }
                    ActionId::GoFirst => { this.file_list.select_first(); return glib::Propagation::Stop; }
                    ActionId::GoLast => { this.file_list.select_last(); return glib::Propagation::Stop; }
                    ActionId::HistoryBack => { this.go_back(); return glib::Propagation::Stop; }
                    ActionId::HistoryForward => { this.go_forward(); return glib::Propagation::Stop; }
                    ActionId::FocusSidebar => { this.sidebar.grab_focus(); return glib::Propagation::Stop; }
                    ActionId::FocusList => { this.file_list.grab_focus(); return glib::Propagation::Stop; }
                }
            }

            let in_sidebar = if let Some(focus) = gtk4::prelude::GtkWindowExt::focus(&this.window) {
                focus.is_ancestor(this.sidebar.widget()) || focus == *this.sidebar.widget()
            } else {
                false
            };

            if in_sidebar {
                match key {
                    Key::Return | Key::KP_Enter => {
                        if this.sidebar.activate_selected() {
                            this.file_list.grab_focus();
                        }
                        return glib::Propagation::Stop;
                    }
                    Key::j => {
                        this.sidebar.select_next();
                        return glib::Propagation::Stop;
                    }
                    Key::k => {
                        this.sidebar.select_prev();
                        return glib::Propagation::Stop;
                    }
                    Key::Right | Key::Escape => {
                        this.file_list.grab_focus();
                        return glib::Propagation::Stop;
                    }
                    _ => {}
                }
            }

            if key == Key::Escape {
                this.file_list.unselect_all();
                this.file_list.grab_focus();
                return glib::Propagation::Stop;
            }

            glib::Propagation::Proceed
        });

        self.window.add_controller(key_controller);
    }

    fn setup_mouse_navigation(self: &Rc<Self>) {
        let mouse_nav = GestureClick::new();
        mouse_nav.set_button(0); // Captura todos los botones, incluidos botones laterales (8 y 9)
        mouse_nav.set_propagation_phase(gtk4::PropagationPhase::Capture);
        let this_nav = self.clone();
        mouse_nav.connect_pressed(move |gesture, _n_press, _x, _y| {
            match gesture.current_button() {
                8 => {
                    gesture.set_state(gtk4::EventSequenceState::Claimed);
                    this_nav.go_back();
                }
                9 => {
                    gesture.set_state(gtk4::EventSequenceState::Claimed);
                    this_nav.go_forward();
                }
                _ => {}
            }
        });
        self.window.add_controller(mouse_nav);
    }

    pub fn navigate_to(&self, target_path: PathBuf, record_history: bool) {
        let path = if let Ok(canonical) = target_path.canonicalize() {
            canonical
        } else {
            target_path
        };

        if !path.is_dir() {
            self.show_toast(&format!("Path {} is not a directory", path.display()));
            return;
        }

        if record_history {
            let cur = self.current_path.borrow().clone();
            if cur != path {
                self.history_back.borrow_mut().push(cur);
                self.history_forward.borrow_mut().clear();
            }
        }

        *self.current_path.borrow_mut() = path.clone();
        self.update_history_buttons();

        // Actualizar barra de ruta
        let display_str = path.display().to_string();
        self.path_btn.set_label(&display_str);
        self.path_entry.set_text(&display_str);
        self.path_stack.set_visible_child_name("button");

        // Actualizar barra de espacio de disco
        self.sidebar.update_disk_space(&path);

        // Limpiar filtro de búsqueda al navegar
        self.search_entry.set_text("");
        self.search_bar.set_search_mode(false);

        // Cargar archivos
        self.refresh();
        self.update_tabs_ui();
    }

    pub fn refresh(&self) {
        let path = self.current_path.borrow().clone();
        let show_hidden = *self.show_hidden.borrow();

        match operations::read_directory(&path, show_hidden) {
            Ok(entries) => {
                let total = entries.len();
                let hidden_count = entries.iter().filter(|e| e.is_hidden).count();
                self.file_list.set_entries(entries);

                let status = if let Some(space) = operations::get_disk_space(&path) {
                    if show_hidden && hidden_count > 0 {
                        format!("{total} items ({hidden_count} hidden)   •   Space: {} free of {}", space.formatted_available, space.formatted_total)
                    } else {
                        format!("{total} items   •   Space: {} free of {}", space.formatted_available, space.formatted_total)
                    }
                } else {
                    if show_hidden && hidden_count > 0 {
                        format!("{total} items ({hidden_count} hidden)")
                    } else {
                        format!("{total} items")
                    }
                };
                self.status_label.set_text(&status);
            }
            Err(e) => {
                self.show_toast(&format!("Error reading directory: {e}"));
                self.file_list.set_entries(Vec::new());
                self.status_label.set_text("Access error");
            }
        }
    }

    pub fn go_parent(&self) {
        let cur = self.current_path.borrow().clone();
        if let Some(parent) = cur.parent() {
            self.navigate_to(parent.to_path_buf(), true);
        }
    }

    pub fn go_back(&self) {
        let prev = self.history_back.borrow_mut().pop();
        if let Some(prev) = prev {
            let cur = self.current_path.borrow().clone();
            self.history_forward.borrow_mut().push(cur);
            self.navigate_to(prev, false);
        }
    }

    pub fn go_forward(&self) {
        let next = self.history_forward.borrow_mut().pop();
        if let Some(next) = next {
            let cur = self.current_path.borrow().clone();
            self.history_back.borrow_mut().push(cur);
            self.navigate_to(next, false);
        }
    }

    fn update_history_buttons(&self) {
        self.btn_back.set_sensitive(!self.history_back.borrow().is_empty());
        self.btn_forward.set_sensitive(!self.history_forward.borrow().is_empty());
    }

    pub fn set_show_hidden(&self, show: bool) {
        *self.show_hidden.borrow_mut() = show;
        self.refresh();
    }

    pub fn activate_path_entry(&self) {
        self.header_revealer.set_reveal_child(true);
        self.path_stack.set_visible_child_name("entry");
        self.path_entry.grab_focus();
        self.path_entry.select_region(0, -1);
    }

    fn handle_path_input(&self, input: &str) {
        let trimmed = input.trim();
        let resolved = if trimmed.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                let rest = trimmed.trim_start_matches('~').trim_start_matches('/');
                home.join(rest)
            } else {
                PathBuf::from(trimmed)
            }
        } else {
            PathBuf::from(trimmed)
        };

        if resolved.exists() {
            self.navigate_to(resolved, true);
        } else {
            self.show_toast(&format!("Path '{}' does not exist", trimmed));
            self.path_stack.set_visible_child_name("button");
        }
        self.file_list.grab_focus();
    }

    pub fn copy_selected(&self, is_cut: bool) {
        let entries = self.file_list.selected_entries();
        if !entries.is_empty() {
            let paths: Vec<PathBuf> = entries.iter().map(|e| e.path.clone()).collect();
            let count = paths.len();
            *self.clipboard.borrow_mut() = Some((paths, is_cut));
            let action_desc = if is_cut { "Cut:" } else { "Copied:" };
            if count == 1 {
                self.show_toast(&format!("{action_desc} {}", entries[0].name));
            } else {
                self.show_toast(&format!("{action_desc} {count} items"));
            }
            self.update_action_bubble();
        }
    }

    pub fn paste_clipboard(self: &Rc<Self>) {
        let clip = self.clipboard.borrow().clone();
        if let Some((src_paths, is_cut)) = clip {
            if src_paths.is_empty() {
                self.show_toast("Clipboard is empty");
                *self.clipboard.borrow_mut() = None;
                self.update_action_bubble();
                return;
            }

            let current = self.current_path.borrow().clone();
            let action_title = if is_cut { "Moving..." } else { "Copying..." };
            let icon_name = if is_cut { "edit-cut-symbolic" } else { "edit-copy-symbolic" };

            let this_done = self.clone();
            let is_cut_mode = is_cut;

            let (sender, cancel_flag) = self.sidebar.start_operation(
                action_title,
                icon_name,
                move |success, res_path| {
                    if success {
                        if is_cut_mode {
                            *this_done.clipboard.borrow_mut() = None;
                        }
                        this_done.refresh();
                        if let Some(p) = res_path {
                            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                                this_done.file_list.select_name(name);
                            }
                        }
                        this_done.update_action_bubble();
                    }
                },
            );

            std::thread::spawn(move || {
                operations::copy_multiple_with_progress(src_paths, current, is_cut, sender, cancel_flag);
            });
        } else {
            self.show_toast("Clipboard is empty");
        }
    }

    pub fn delete_selected(self: &Rc<Self>) {
        if operations::is_trash_path(&self.current_path.borrow()) {
            self.delete_selected_permanently();
            return;
        }

        let entries = self.file_list.selected_entries();
        if !entries.is_empty() {
            let this = self.clone();
            let paths: Vec<PathBuf> = entries.iter().map(|e| e.path.clone()).collect();
            let count = paths.len();
            let prompt_name = if count == 1 {
                entries[0].name.clone()
            } else {
                format!("{count} items")
            };
            let toast_name = prompt_name.clone();

            confirm_delete(&self.window, &prompt_name, move || {
                let mut errors = 0;
                for p in &paths {
                    if let Err(e) = operations::trash_item(p) {
                        errors += 1;
                        this.show_toast(&e);
                    }
                }
                if errors == 0 {
                    this.show_toast(&format!("\"{toast_name}\" moved to trash"));
                }
                this.refresh();
                this.update_action_bubble();
            });
        }
    }

    pub fn delete_selected_permanently(self: &Rc<Self>) {
        let entries = self.file_list.selected_entries();
        if !entries.is_empty() {
            let this = self.clone();
            let paths: Vec<PathBuf> = entries.iter().map(|e| e.path.clone()).collect();
            let count = paths.len();
            let prompt_name = if count == 1 {
                entries[0].name.clone()
            } else {
                format!("{count} items")
            };
            let toast_name = prompt_name.clone();

            crate::ui::dialogs::confirm_permanent_delete(&self.window, &prompt_name, move || {
                match operations::permanently_delete_items(&paths) {
                    Ok(_) => {
                        this.show_toast(&format!("\"{toast_name}\" permanently deleted"));
                    }
                    Err(e) => {
                        this.show_toast(&e);
                    }
                }
                this.refresh();
                this.update_action_bubble();
            });
        }
    }

    pub fn restore_selected(self: &Rc<Self>) {
        let entries = self.file_list.selected_entries();
        if !entries.is_empty() {
            let this = self.clone();
            let paths: Vec<PathBuf> = entries.iter().map(|e| e.path.clone()).collect();
            match operations::restore_trash_items(&paths) {
                Ok(restored) => {
                    this.show_toast(&format!("{restored} item(s) restored"));
                    this.refresh();
                    this.update_action_bubble();
                }
                Err(e) => {
                    this.show_toast(&e);
                }
            }
        }
    }

    pub fn empty_trash(self: &Rc<Self>) {
        let this = self.clone();
        crate::ui::dialogs::confirm_empty_trash(&self.window, move || {
            match operations::empty_trash() {
                Ok(_) => {
                    this.show_toast("Trash emptied");
                    this.refresh();
                    this.update_action_bubble();
                }
                Err(e) => {
                    this.show_toast(&e);
                }
            }
        });
    }

    pub fn rename_selected(self: &Rc<Self>) {
        if let Some(entry) = self.file_list.selected_entry() {
            let this = self.clone();
            let path = entry.path.clone();
            let old_name = entry.name.clone();
            let old_name_check = old_name.clone();

            prompt_input(
                &self.window,
                "Rename Item",
                "New name...",
                &old_name,
                "Rename",
                move |new_name| {
                    if new_name == old_name_check {
                        return;
                    }
                    match operations::rename_item(&path, &new_name) {
                        Ok(_) => {
                            this.show_toast(&format!("Renamed to \"{new_name}\""));
                            this.refresh();
                            this.file_list.select_name(&new_name);
                        }
                        Err(e) => this.show_toast(&e),
                    }
                },
            );
        }
    }

    pub fn new_file(self: &Rc<Self>) {
        let this = self.clone();
        let current = self.current_path.borrow().clone();

        prompt_input(
            &self.window,
            "New File",
            "nombre.txt",
            "",
            "Create File",
            move |name| {
                match operations::create_file(&current, &name) {
                    Ok(_) => {
                        this.show_toast(&format!("File created: {name}"));
                        this.refresh();
                        this.file_list.select_name(&name);
                    }
                    Err(e) => this.show_toast(&e),
                }
            },
        );
    }

    pub fn new_folder(self: &Rc<Self>) {
        let this = self.clone();
        let current = self.current_path.borrow().clone();

        prompt_input(
            &self.window,
            "New Folder",
            "Folder name...",
            "",
            "Create Folder",
            move |name| {
                match operations::create_folder(&current, &name) {
                    Ok(_) => {
                        this.show_toast(&format!("Folder created: {name}"));
                        this.refresh();
                        this.file_list.select_name(&name);
                    }
                    Err(e) => this.show_toast(&e),
                }
            },
        );
    }

    pub fn open_terminal(&self) {
        let current = self.current_path.borrow().clone();
        match operations::open_in_terminal(&current) {
            Ok(_) => self.show_toast("Terminal opened"),
            Err(e) => self.show_toast(&e),
        }
    }

    pub fn open_shortcuts_dialog(self: &Rc<Self>) {
        let this = self.clone();
        let sc = self.shortcuts.clone();
        crate::ui::shortcuts_dialog::show_shortcuts_dialog(&self.window, sc, move || {
            this.show_toast("Keyboard shortcuts updated");
        });
    }

    pub fn open_settings(self: &Rc<Self>) {
        let this = self.clone();
        let settings = self.settings.clone();
        let sc = self.shortcuts.clone();
        show_settings_dialog(&self.window, settings, sc, move |new_settings| {
            this.header_revealer.set_reveal_child(!new_settings.auto_hide_navbar);
            this.sidebar.set_collapsed(new_settings.sidebar_collapsed);
            this.paned_main.set_position(if new_settings.sidebar_collapsed { 48 } else { 160 });
            this.preview_pane.widget().set_visible(new_settings.show_preview);

            let cur_hidden = *this.show_hidden.borrow();
            if cur_hidden != new_settings.show_hidden {
                *this.show_hidden.borrow_mut() = new_settings.show_hidden;
                this.refresh();
            }

            let cur_grid = this.file_list.is_grid();
            if cur_grid != new_settings.is_grid_view {
                this.file_list.set_view_mode(if new_settings.is_grid_view {
                    ViewMode::Grid
                } else {
                    ViewMode::List
                });
                this.btn_view_mode.set_icon_name(if new_settings.is_grid_view { "view-list-symbolic" } else { "view-grid-symbolic" });
                this.btn_view_mode.set_tooltip_text(Some(if new_settings.is_grid_view { "Cambiar a vista de lista (v)" } else { "Cambiar a vista de cuadrícula (v)" }));
            }

            if new_settings.compact_navbar {
                this.header.add_css_class("compact-navbar");
            } else {
                this.header.remove_css_class("compact-navbar");
            }
            crate::ui::theme::apply_icon_theme(&new_settings.icon_theme);
            crate::ui::theme::apply_theme_css(new_settings.corner_roundness, new_settings.window_opacity);
        });
    }

    pub fn toggle_preview(self: &Rc<Self>) {
        let is_vis = self.preview_pane.widget().is_visible();
        let new_val = !is_vis;
        self.preview_pane.widget().set_visible(new_val);
        self.settings.borrow_mut().show_preview = new_val;
        self.settings.borrow().save();
        if new_val {
            self.show_toast("Preview pane enabled");
        } else {
            self.show_toast("Preview pane hidden");
        }
    }

    pub fn copy_current_path(&self) {
        let path_str = self.current_path.borrow().display().to_string();
        self.window.clipboard().set_text(&path_str);
        self.show_toast(&format!("Path copied: {path_str}"));
    }

    pub fn toggle_show_hidden(self: &Rc<Self>) {
        let cur = *self.show_hidden.borrow();
        let new_val = !cur;
        self.set_show_hidden(new_val);
        self.settings.borrow_mut().show_hidden = new_val;
        self.settings.borrow().save();
        if new_val {
            self.show_toast("Hidden files visible");
        } else {
            self.show_toast("Hidden files hidden");
        }
    }

    pub fn toggle_navbar(self: &Rc<Self>) {
        let is_revealed = self.header_revealer.reveals_child();
        self.header_revealer.set_reveal_child(!is_revealed);
        if !is_revealed {
            self.show_toast("Navigation bar visible");
        } else {
            self.show_toast("Navigation bar hidden (Press F10 to toggle)");
        }
    }

    pub fn toggle_view_mode(self: &Rc<Self>) {
        let new_mode = self.file_list.toggle_view_mode();
        let is_grid = new_mode == ViewMode::Grid;
        self.settings.borrow_mut().is_grid_view = is_grid;
        self.settings.borrow().save();
        self.btn_view_mode.set_icon_name(if is_grid { "view-list-symbolic" } else { "view-grid-symbolic" });
        self.btn_view_mode.set_tooltip_text(Some(if is_grid { "Cambiar a vista de lista (v)" } else { "Cambiar a vista de cuadrícula (v)" }));
        if is_grid {
            self.show_toast("Grid view (cards)");
        } else {
            self.show_toast("List view");
        }
    }


    pub fn zoom_in(self: &Rc<Self>) {
        self.file_list.zoom_in();
        self.settings.borrow_mut().zoom_level = self.file_list.current_zoom();
        self.settings.borrow().save();
        self.show_toast(&format!("Zoom: {}%", self.file_list.zoom_percent()));
    }

    pub fn zoom_out(self: &Rc<Self>) {
        self.file_list.zoom_out();
        self.settings.borrow_mut().zoom_level = self.file_list.current_zoom();
        self.settings.borrow().save();
        self.show_toast(&format!("Zoom: {}%", self.file_list.zoom_percent()));
    }

    pub fn zoom_reset(self: &Rc<Self>) {
        self.file_list.zoom_reset();
        self.settings.borrow_mut().zoom_level = 0;
        self.settings.borrow().save();
        self.show_toast("Zoom: 100%");
    }

    pub fn open_command_palette(self: &Rc<Self>) {
        let this = self.clone();
        let current = self.current_path.borrow().clone();
        let sc = self.shortcuts.borrow();

        show_command_palette(&self.window, current, &sc, move |action| {
            match action {
                PaletteAction::Navigate(p) => this.navigate_to(p, true),
                PaletteAction::ToggleHidden => this.toggle_show_hidden(),
                PaletteAction::CopyPath => this.copy_current_path(),
                PaletteAction::TogglePreview => this.toggle_preview(),
                PaletteAction::ToggleSidebar => this.toggle_sidebar_collapse(),
                PaletteAction::ToggleNavbar => this.toggle_navbar(),
                PaletteAction::OpenTerminal => this.open_terminal(),
                PaletteAction::NewFile => this.new_file(),
                PaletteAction::NewFolder => this.new_folder(),
                PaletteAction::Copy => this.copy_selected(false),
                PaletteAction::Cut => this.copy_selected(true),
                PaletteAction::Paste => this.paste_clipboard(),
                PaletteAction::Extract => this.extract_selected(),
                PaletteAction::Compress => this.compress_selected(),
                PaletteAction::Delete => this.delete_selected(),
                PaletteAction::Rename => this.rename_selected(),
                PaletteAction::OpenSettings => this.open_settings(),
                PaletteAction::SelectNext => this.file_list.select_next(),
                PaletteAction::SelectPrev => this.file_list.select_prev(),
                PaletteAction::OpenSelected => {
                    if let Some(e) = this.file_list.selected_entry() {
                        this.open_entry(&e);
                    }
                }
                PaletteAction::GoParent => this.go_parent(),
                PaletteAction::GoFirst => this.file_list.select_first(),
                PaletteAction::GoLast => this.file_list.select_last(),
                PaletteAction::GoBack => this.go_back(),
                PaletteAction::GoForward => this.go_forward(),
                PaletteAction::EditPath => this.activate_path_entry(),
                PaletteAction::SearchFilter => {
                    this.search_bar.set_search_mode(true);
                    this.search_entry.grab_focus();
                }
                PaletteAction::ToggleViewMode => this.toggle_view_mode(),
                PaletteAction::ZoomIn => this.zoom_in(),
                PaletteAction::ZoomOut => this.zoom_out(),
                PaletteAction::ZoomReset => this.zoom_reset(),
                PaletteAction::NewTab => this.open_new_tab_for_selection(),
                PaletteAction::CloseTab => this.close_current_tab(),
                PaletteAction::NextTab => this.next_tab(),
                PaletteAction::PrevTab => this.prev_tab(),
                PaletteAction::OpenShortcuts => this.open_shortcuts_dialog(),
                PaletteAction::ShowProgress => this.sidebar.toggle_progress_popover(),
                PaletteAction::SetTheme(theme_id) => {
                    crate::ui::theme::set_mrdemonc_theme(&theme_id);
                    crate::ui::theme::reload_theme();
                }
            }
        });
    }


    // ==========================================
    // SISTEMA DE PESTAÑAS (TABS)
    // ==========================================

    pub fn open_new_tab_for_selection(&self) {
        // 1. Si la barra lateral tiene el foco o un elemento seleccionado:
        if self.sidebar.has_focus() {
            if let Some(path) = self.sidebar.selected_path() {
                self.open_new_tab(path);
                return;
            }
        }

        // 2. Si un directorio está seleccionado en la lista:
        if let Some(entry) = self.file_list.selected_entry() {
            if entry.is_dir {
                self.open_new_tab(entry.path);
                return;
            }
        }

        // 3. De lo contrario, duplicar la carpeta actual en nueva pestaña:
        let cur = self.current_path.borrow().clone();
        self.open_new_tab(cur);
    }

    pub fn open_new_tab(&self, target_path: PathBuf) {
        let cur_idx = *self.active_tab.borrow();
        let cur_path = self.current_path.borrow().clone();
        let cur_back = self.history_back.borrow().clone();
        let cur_fwd = self.history_forward.borrow().clone();

        if let Some(tab) = self.tabs.borrow_mut().get_mut(cur_idx) {
            tab.path = cur_path;
            tab.history_back = cur_back;
            tab.history_forward = cur_fwd;
        }

        let new_tab = TabState {
            path: target_path.clone(),
            history_back: Vec::new(),
            history_forward: Vec::new(),
        };

        self.tabs.borrow_mut().push(new_tab);
        let new_idx = self.tabs.borrow().len() - 1;
        *self.active_tab.borrow_mut() = new_idx;

        self.history_back.borrow_mut().clear();
        self.history_forward.borrow_mut().clear();
        self.navigate_to(target_path, false);
        self.update_tabs_ui();

        let name = self.current_path.borrow()
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string());
        self.show_toast(&format!("Tab {}: {name}", new_idx + 1));
    }

    pub fn switch_to_tab(&self, idx: usize) {
        let tabs_count = self.tabs.borrow().len();
        if idx >= tabs_count {
            return;
        }
        let cur_idx = *self.active_tab.borrow();
        if idx == cur_idx {
            return;
        }

        let cur_path = self.current_path.borrow().clone();
        let cur_back = self.history_back.borrow().clone();
        let cur_fwd = self.history_forward.borrow().clone();
        if let Some(tab) = self.tabs.borrow_mut().get_mut(cur_idx) {
            tab.path = cur_path;
            tab.history_back = cur_back;
            tab.history_forward = cur_fwd;
        }

        *self.active_tab.borrow_mut() = idx;

        let (target_path, back, fwd) = {
            let tabs = self.tabs.borrow();
            let tab = &tabs[idx];
            (tab.path.clone(), tab.history_back.clone(), tab.history_forward.clone())
        };

        *self.history_back.borrow_mut() = back;
        *self.history_forward.borrow_mut() = fwd;
        self.navigate_to(target_path, false);
        self.update_tabs_ui();
    }

    pub fn close_tab(&self, idx: usize) {
        let tabs_count = self.tabs.borrow().len();
        if tabs_count <= 1 || idx >= tabs_count {
            return;
        }

        let cur_idx = *self.active_tab.borrow();
        self.tabs.borrow_mut().remove(idx);

        let new_idx = if cur_idx == idx {
            if idx >= self.tabs.borrow().len() {
                self.tabs.borrow().len() - 1
            } else {
                idx
            }
        } else if cur_idx > idx {
            cur_idx - 1
        } else {
            cur_idx
        };

        *self.active_tab.borrow_mut() = new_idx;

        if cur_idx == idx {
            let (target_path, back, fwd) = {
                let tabs = self.tabs.borrow();
                let tab = &tabs[new_idx];
                (tab.path.clone(), tab.history_back.clone(), tab.history_forward.clone())
            };
            *self.history_back.borrow_mut() = back;
            *self.history_forward.borrow_mut() = fwd;
            self.navigate_to(target_path, false);
        }

        self.update_tabs_ui();
    }

    pub fn close_current_tab(&self) {
        let cur = *self.active_tab.borrow();
        self.close_tab(cur);
    }

    pub fn next_tab(&self) {
        let count = self.tabs.borrow().len();
        if count > 1 {
            let cur = *self.active_tab.borrow();
            let next = (cur + 1) % count;
            self.switch_to_tab(next);
        }
    }

    pub fn prev_tab(&self) {
        let count = self.tabs.borrow().len();
        if count > 1 {
            let cur = *self.active_tab.borrow();
            let prev = if cur == 0 { count - 1 } else { cur - 1 };
            self.switch_to_tab(prev);
        }
    }

    pub fn update_tabs_ui(&self) {
        let cur_idx = *self.active_tab.borrow();
        let cur_path = self.current_path.borrow().clone();
        if let Some(tab) = self.tabs.borrow_mut().get_mut(cur_idx) {
            tab.path = cur_path;
        }

        let tabs_info: Vec<TabInfo> = self.tabs.borrow().iter().enumerate().map(|(i, t)| {
            let title = t.path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| {
                    if t.path == PathBuf::from("/") {
                        "Root".to_string()
                    } else {
                        t.path.display().to_string()
                    }
                });
            TabInfo {
                path: t.path.clone(),
                title,
                is_active: i == cur_idx,
            }
        }).collect();

        self.tabs_bubble.set_tabs(&tabs_info);
    }

    pub fn show_toast(&self, message: &str) {
        let toast = adw::Toast::new(message);
        toast.set_timeout(3);
        self.toast_overlay.add_toast(toast);
    }
}
