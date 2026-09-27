use gtk4::gdk;
use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Image, Label, Orientation, Popover, Separator};
use std::cell::RefCell;
use std::rc::Rc;
use crate::fs::entry::FileEntry;

#[derive(Clone)]
pub struct ContextMenuActions {
    #[allow(dead_code)]
    pub on_open: Rc<dyn Fn()>,
    pub on_new_tab: Rc<dyn Fn()>,
    pub on_copy: Rc<dyn Fn()>,
    pub on_cut: Rc<dyn Fn()>,
    pub on_paste: Rc<dyn Fn()>,
    pub on_rename: Rc<dyn Fn()>,
    pub on_extract: Rc<dyn Fn()>,
    pub on_compress: Rc<dyn Fn()>,
    pub on_trash: Rc<dyn Fn()>,
    pub on_delete_permanently: Rc<dyn Fn()>,
    pub on_restore_trash: Rc<dyn Fn()>,
    pub on_empty_trash: Rc<dyn Fn()>,
    pub on_new_file: Rc<dyn Fn()>,
    pub on_new_folder: Rc<dyn Fn()>,
    pub on_terminal: Rc<dyn Fn()>,
    pub on_copy_path: Rc<dyn Fn()>,
    pub on_select_all: Rc<dyn Fn()>,
    pub on_refresh: Rc<dyn Fn()>,
}

#[derive(Clone)]
pub struct ContextMenu {
    popover: Popover,
    container: Box,
    actions: Rc<RefCell<Option<ContextMenuActions>>>,
}

impl ContextMenu {
    pub fn new<W: IsA<gtk4::Widget>>(parent: &W) -> Self {
        let container = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(1)
            .css_classes(["context-menu-box"])
            .build();

        let popover = Popover::builder()
            .autohide(true)
            .has_arrow(true)
            .child(&container)
            .css_classes(["menu", "context-menu-popover"])
            .build();

        popover.set_parent(parent);

        Self {
            popover,
            container,
            actions: Rc::new(RefCell::new(None)),
        }
    }

    pub fn set_actions(&self, actions: ContextMenuActions) {
        *self.actions.borrow_mut() = Some(actions);
    }

    pub fn show_for_item(&self, x: f64, y: f64, selected: &[FileEntry], has_clipboard: bool, in_trash: bool) {
        while let Some(child) = self.container.first_child() {
            self.container.remove(&child);
        }

        let count = selected.len();
        let is_single = count == 1;
        let is_dir = if is_single { selected[0].is_dir } else { false };
        let is_archive = if is_single { selected[0].is_archive } else { false };

        let actions_ref = self.actions.borrow();
        let actions = match actions_ref.as_ref() {
            Some(a) => a.clone(),
            None => return,
        };

        if in_trash {
            // 1. Restaurar
            let on_restore = actions.on_restore_trash.clone();
            let pop = self.popover.clone();
            let restore_label = if count > 1 { format!("Restaurar {count} elementos") } else { "Restaurar".to_string() };
            self.container.append(&create_menu_item(
                "edit-undo-symbolic",
                &restore_label,
                "",
                false,
                &pop,
                move || on_restore(),
            ));

            self.container.append(&create_separator());

            // 2. Copiar ruta
            let on_copy_path = actions.on_copy_path.clone();
            let pop = self.popover.clone();
            self.container.append(&create_menu_item(
                "edit-copy-symbolic",
                "Copiar ruta",
                "Ctrl+Shift+C",
                false,
                &pop,
                move || on_copy_path(),
            ));

            self.container.append(&create_separator());

            // 3. Eliminar definitivamente
            let on_perm_del = actions.on_delete_permanently.clone();
            let pop = self.popover.clone();
            let del_label = if count > 1 { format!("Eliminar {count} permanentemente") } else { "Eliminar definitivamente".to_string() };
            self.container.append(&create_menu_item(
                "edit-delete-symbolic",
                &del_label,
                "Shift+Supr",
                true,
                &pop,
                move || on_perm_del(),
            ));

            // 4. Vaciar papelera
            let on_empty = actions.on_empty_trash.clone();
            let pop = self.popover.clone();
            self.container.append(&create_menu_item(
                "user-trash-symbolic",
                "Vaciar papelera",
                "",
                true,
                &pop,
                move || on_empty(),
            ));

            let rect = gdk::Rectangle::new(x as i32, y as i32, 1, 1);
            self.popover.set_pointing_to(Some(&rect));
            self.popover.popup();
            return;
        }

        // --- GRUPO 1: Pestaña y renombre ---
        let mut has_group1 = false;

        // 1. Abrir en nueva pestaña (si es carpeta)
        if is_dir {
            let on_tab = actions.on_new_tab.clone();
            let pop = self.popover.clone();
            self.container.append(&create_menu_item(
                "tab-new-symbolic",
                "Abrir en nueva pestaña",
                "Ctrl+T",
                false,
                &pop,
                move || on_tab(),
            ));
            has_group1 = true;
        }

        // 2. Renombrar (si es elemento único)
        if is_single {
            let on_rename = actions.on_rename.clone();
            let pop = self.popover.clone();
            self.container.append(&create_menu_item(
                "document-edit-symbolic",
                "Renombrar",
                "F2",
                false,
                &pop,
                move || on_rename(),
            ));
            has_group1 = true;
        }

        if has_group1 {
            self.container.append(&create_separator());
        }

        // --- GRUPO 2: Portapapeles, compresión y utilidades ---
        // 4. Cortar
        let on_cut = actions.on_cut.clone();
        let pop = self.popover.clone();
        let cut_label = if count > 1 { format!("Cortar {count} elementos") } else { "Cortar".to_string() };
        self.container.append(&create_menu_item(
            "edit-cut-symbolic",
            &cut_label,
            "Ctrl+X",
            false,
            &pop,
            move || on_cut(),
        ));

        // 5. Copiar
        let on_copy = actions.on_copy.clone();
        let pop = self.popover.clone();
        let copy_label = if count > 1 { format!("Copiar {count} elementos") } else { "Copiar".to_string() };
        self.container.append(&create_menu_item(
            "edit-copy-symbolic",
            &copy_label,
            "Ctrl+C",
            false,
            &pop,
            move || on_copy(),
        ));

        // 6. Pegar (si es directorio único y hay portapapeles)
        if is_dir && has_clipboard {
            let on_paste = actions.on_paste.clone();
            let pop = self.popover.clone();
            self.container.append(&create_menu_item(
                "edit-paste-symbolic",
                "Pegar en esta carpeta",
                "Ctrl+V",
                false,
                &pop,
                move || on_paste(),
            ));
        }

        // 7. Extraer (si es archivo comprimido)
        if is_archive {
            let on_extract = actions.on_extract.clone();
            let pop = self.popover.clone();
            self.container.append(&create_menu_item(
                "package-x-generic",
                "Extraer aquí",
                "e",
                false,
                &pop,
                move || on_extract(),
            ));
        }

        // 8. Comprimir a ZIP
        let on_compress = actions.on_compress.clone();
        let pop = self.popover.clone();
        self.container.append(&create_menu_item(
            "package-x-generic-symbolic",
            "Comprimir a ZIP",
            "z",
            false,
            &pop,
            move || on_compress(),
        ));

        // 9. Copiar ruta
        let on_copy_path = actions.on_copy_path.clone();
        let pop = self.popover.clone();
        self.container.append(&create_menu_item(
            "edit-copy-symbolic",
            "Copiar ruta",
            "Ctrl+Shift+C",
            false,
            &pop,
            move || on_copy_path(),
        ));

        // 10. Abrir en terminal (si es directorio)
        if is_dir {
            let on_term = actions.on_terminal.clone();
            let pop = self.popover.clone();
            self.container.append(&create_menu_item(
                "utilities-terminal-symbolic",
                "Abrir terminal aquí",
                "o",
                false,
                &pop,
                move || on_term(),
            ));
        }

        self.container.append(&create_separator());

        // --- GRUPO 3: Eliminación ---
        // 11. Mover a la papelera
        let on_trash = actions.on_trash.clone();
        let pop = self.popover.clone();
        let trash_label = if count > 1 { format!("Mover {count} a la papelera") } else { "Mover a la papelera".to_string() };
        self.container.append(&create_menu_item(
            "user-trash-symbolic",
            &trash_label,
            "Supr",
            true,
            &pop,
            move || on_trash(),
        ));

        // 12. Eliminar permanentemente
        let on_perm_del = actions.on_delete_permanently.clone();
        let pop = self.popover.clone();
        let perm_label = if count > 1 { format!("Eliminar {count} definitivamente") } else { "Eliminar definitivamente".to_string() };
        self.container.append(&create_menu_item(
            "edit-delete-symbolic",
            &perm_label,
            "Shift+Supr",
            true,
            &pop,
            move || on_perm_del(),
        ));

        let rect = gdk::Rectangle::new(x as i32, y as i32, 1, 1);
        self.popover.set_pointing_to(Some(&rect));
        self.popover.popup();
    }

    pub fn show_for_empty(&self, x: f64, y: f64, has_clipboard: bool, in_trash: bool) {
        while let Some(child) = self.container.first_child() {
            self.container.remove(&child);
        }

        let actions_ref = self.actions.borrow();
        let actions = match actions_ref.as_ref() {
            Some(a) => a.clone(),
            None => return,
        };

        if in_trash {
            // 1. Vaciar papelera
            let on_empty = actions.on_empty_trash.clone();
            let pop = self.popover.clone();
            self.container.append(&create_menu_item(
                "user-trash-symbolic",
                "Vaciar papelera",
                "",
                true,
                &pop,
                move || on_empty(),
            ));

            self.container.append(&create_separator());

            // 2. Seleccionar todo
            let on_sel_all = actions.on_select_all.clone();
            let pop = self.popover.clone();
            self.container.append(&create_menu_item(
                "edit-select-all-symbolic",
                "Seleccionar todo",
                "Ctrl+A",
                false,
                &pop,
                move || on_sel_all(),
            ));

            // 3. Recargar
            let on_ref = actions.on_refresh.clone();
            let pop = self.popover.clone();
            self.container.append(&create_menu_item(
                "view-refresh-symbolic",
                "Recargar",
                "F5",
                false,
                &pop,
                move || on_ref(),
            ));

            let rect = gdk::Rectangle::new(x as i32, y as i32, 1, 1);
            self.popover.set_pointing_to(Some(&rect));
            self.popover.popup();
            return;
        }

        // 1. Nueva carpeta
        let on_new_folder = actions.on_new_folder.clone();
        let pop = self.popover.clone();
        self.container.append(&create_menu_item(
            "folder-new-symbolic",
            "Nueva carpeta",
            "Ctrl+Shift+N",
            false,
            &pop,
            move || on_new_folder(),
        ));

        // 2. Nuevo archivo
        let on_new_file = actions.on_new_file.clone();
        let pop = self.popover.clone();
        self.container.append(&create_menu_item(
            "document-new-symbolic",
            "Nuevo archivo",
            "Ctrl+N",
            false,
            &pop,
            move || on_new_file(),
        ));

        // 3. Pegar
        let on_paste = actions.on_paste.clone();
        let pop = self.popover.clone();
        let paste_btn = create_menu_item(
            "edit-paste-symbolic",
            "Pegar",
            "Ctrl+V",
            false,
            &pop,
            move || on_paste(),
        );
        paste_btn.set_sensitive(has_clipboard);
        self.container.append(&paste_btn);

        self.container.append(&create_separator());

        // 4. Abrir en terminal
        let on_term = actions.on_terminal.clone();
        let pop = self.popover.clone();
        self.container.append(&create_menu_item(
            "utilities-terminal-symbolic",
            "Abrir terminal aquí",
            "o",
            false,
            &pop,
            move || on_term(),
        ));

        // 5. Copiar ruta
        let on_copy_path = actions.on_copy_path.clone();
        let pop = self.popover.clone();
        self.container.append(&create_menu_item(
            "edit-copy-symbolic",
            "Copiar ruta de carpeta",
            "Ctrl+Shift+C",
            false,
            &pop,
            move || on_copy_path(),
        ));

        self.container.append(&create_separator());

        // 6. Seleccionar todo
        let on_sel_all = actions.on_select_all.clone();
        let pop = self.popover.clone();
        self.container.append(&create_menu_item(
            "edit-select-all-symbolic",
            "Seleccionar todo",
            "Ctrl+A",
            false,
            &pop,
            move || on_sel_all(),
        ));

        // 7. Recargar
        let on_ref = actions.on_refresh.clone();
        let pop = self.popover.clone();
        self.container.append(&create_menu_item(
            "view-refresh-symbolic",
            "Recargar carpeta",
            "F5",
            false,
            &pop,
            move || on_ref(),
        ));

        let rect = gdk::Rectangle::new(x as i32, y as i32, 1, 1);
        self.popover.set_pointing_to(Some(&rect));
        self.popover.popup();
    }
}

fn create_separator() -> Separator {
    Separator::builder()
        .orientation(Orientation::Horizontal)
        .css_classes(["context-menu-separator"])
        .build()
}

fn create_menu_item(
    icon_name: &str,
    label_text: &str,
    shortcut_text: &str,
    is_destructive: bool,
    popover: &Popover,
    on_click: impl Fn() + 'static,
) -> Button {
    let btn = Button::builder()
        .css_classes(["flat", "menu-item-btn"])
        .build();

    let h_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .margin_start(4)
        .margin_end(4)
        .margin_top(1)
        .margin_bottom(1)
        .build();

    let icon = Image::builder()
        .icon_name(icon_name)
        .pixel_size(15)
        .build();

    let label = Label::builder()
        .label(label_text)
        .halign(Align::Start)
        .hexpand(true)
        .build();

    if is_destructive {
        btn.add_css_class("destructive-action");
    }

    h_box.append(&icon);
    h_box.append(&label);

    if !shortcut_text.is_empty() {
        let shortcut = Label::builder()
            .label(shortcut_text)
            .halign(Align::End)
            .css_classes(["dim-label", "caption", "menu-item-shortcut"])
            .build();
        h_box.append(&shortcut);
    }

    btn.set_child(Some(&h_box));

    let pop_c = popover.clone();
    btn.connect_clicked(move |_| {
        pop_c.popdown();
        on_click();
    });

    btn
}
