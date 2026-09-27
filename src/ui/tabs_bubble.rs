use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Label, Orientation, pango::EllipsizeMode};

#[derive(Clone, Debug)]
pub struct TabInfo {
    pub path: PathBuf,
    pub title: String,
    pub is_active: bool,
}

#[derive(Clone)]
pub struct TabsBubble {
    container: Box,
    tabs_box: Box,
    btn_add_tab: Button,
    on_select: Rc<RefCell<Option<Rc<dyn Fn(usize)>>>>,
    on_close: Rc<RefCell<Option<Rc<dyn Fn(usize)>>>>,
}

impl TabsBubble {
    pub fn new() -> Self {
        let container = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(4)
            .halign(Align::Center)
            .valign(Align::End)
            .css_classes(["tabs-bubble"])
            .build();

        let tabs_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(4)
            .build();

        let btn_add_tab = Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text("New tab (Ctrl+T)")
            .css_classes(["flat", "tab-add-btn"])
            .build();

        container.append(&tabs_box);
        container.append(&btn_add_tab);

        // Oculto por defecto cuando solo hay 1 pestaña
        container.set_visible(false);

        Self {
            container,
            tabs_box,
            btn_add_tab,
            on_select: Rc::new(RefCell::new(None)),
            on_close: Rc::new(RefCell::new(None)),
        }
    }

    pub fn widget(&self) -> &Box {
        &self.container
    }

    pub fn connect_new_tab<F: Fn() + 'static>(&self, f: F) {
        self.btn_add_tab.connect_clicked(move |_| f());
    }

    pub fn connect_tab_selected<F: Fn(usize) + 'static>(&self, f: F) {
        *self.on_select.borrow_mut() = Some(Rc::new(f));
    }

    pub fn connect_tab_closed<F: Fn(usize) + 'static>(&self, f: F) {
        *self.on_close.borrow_mut() = Some(Rc::new(f));
    }

    pub fn set_tabs(&self, tabs: &[TabInfo]) {
        // Limpiar pestañas anteriores
        while let Some(child) = self.tabs_box.first_child() {
            self.tabs_box.remove(&child);
        }

        // Mostrar solo si hay más de 1 pestaña abierta
        if tabs.len() <= 1 {
            self.container.set_visible(false);
            return;
        }

        self.container.set_visible(true);

        for (idx, tab) in tabs.iter().enumerate() {
            let tab_box = Box::builder()
                .orientation(Orientation::Horizontal)
                .spacing(5)
                .build();

            // Badge numérico del atajo Ctrl+N
            if idx < 9 {
                let badge = Label::builder()
                    .label(&format!("{}", idx + 1))
                    .css_classes(["bubble-shortcut-badge", "tab-shortcut-badge"])
                    .build();
                tab_box.append(&badge);
            }

            // Icono de carpeta
            let icon = gtk4::Image::builder()
                .icon_name("folder-symbolic")
                .pixel_size(14)
                .build();
            tab_box.append(&icon);

            // Nombre de la pestaña
            let label = Label::builder()
                .label(&tab.title)
                .max_width_chars(16)
                .ellipsize(EllipsizeMode::Middle)
                .css_classes(["tab-label"])
                .build();
            tab_box.append(&label);

            let tab_btn = Button::builder()
                .child(&tab_box)
                .tooltip_text(&format!("{} (Ctrl+{})", tab.path.display(), idx + 1))
                .css_classes(if tab.is_active {
                    vec!["tab-pill-btn", "tab-active", "flat"]
                } else {
                    vec!["tab-pill-btn", "tab-inactive", "flat"]
                })
                .build();

            let sel_cb = self.on_select.clone();
            tab_btn.connect_clicked(move |_| {
                if let Some(ref cb) = *sel_cb.borrow() {
                    cb(idx);
                }
            });

            // Botón cerrar pestaña
            let btn_close = Button::builder()
                .icon_name("window-close-symbolic")
                .tooltip_text("Close tab (Ctrl+W)")
                .css_classes(["flat", "tab-close-btn"])
                .build();

            let close_cb = self.on_close.clone();
            btn_close.connect_clicked(move |_| {
                if let Some(ref cb) = *close_cb.borrow() {
                    cb(idx);
                }
            });

            let single_tab_wrapper = Box::builder()
                .orientation(Orientation::Horizontal)
                .spacing(2)
                .css_classes(if tab.is_active {
                    vec!["tab-wrapper", "tab-wrapper-active"]
                } else {
                    vec!["tab-wrapper"]
                })
                .build();

            single_tab_wrapper.append(&tab_btn);
            single_tab_wrapper.append(&btn_close);

            self.tabs_box.append(&single_tab_wrapper);
        }
    }
}
