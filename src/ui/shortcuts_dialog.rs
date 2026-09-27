use libadwaita as adw;
use libadwaita::prelude::*;
use gtk4::{Align, Box, Button, EventControllerKey, HeaderBar, Orientation, ScrolledWindow, SearchEntry};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::keybindings::{ActionId, ActionMeta, ShortcutsManager, ALL_ACTIONS};

pub fn show_shortcuts_dialog<W: IsA<gtk4::Window>, F>(
    parent: &W,
    shortcuts: Rc<RefCell<ShortcutsManager>>,
    on_changed: F,
) where
    F: Fn() + 'static + Clone,
{
    let window = adw::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title("Keyboard Shortcuts & Commands")
        .default_width(580)
        .default_height(680)
        .build();

    let toast_overlay = adw::ToastOverlay::new();

    let root = Box::builder()
        .orientation(Orientation::Vertical)
        .build();

    let header = HeaderBar::new();

    // Botón para restaurar todos los atajos originales
    let btn_reset_all = Button::builder()
        .label("Reset to Defaults")
        .icon_name("view-refresh-symbolic")
        .tooltip_text("Restore all commands to default factory shortcuts")
        .build();
    header.pack_end(&btn_reset_all);
    root.append(&header);

    // Barra de búsqueda para filtrar atajos
    let search_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .margin_start(16)
        .margin_end(16)
        .margin_top(8)
        .margin_bottom(8)
        .build();

    let search_entry = SearchEntry::builder()
        .placeholder_text("Search shortcut or command...")
        .hexpand(true)
        .build();
    search_box.append(&search_entry);
    root.append(&search_box);

    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true)
        .build();

    let page = adw::PreferencesPage::new();

    // Estructura para almacenar callbacks de actualización visual de filas
    type RowRefresher = Rc<dyn Fn()>;
    let row_refreshers: Rc<RefCell<Vec<RowRefresher>>> = Rc::new(RefCell::new(Vec::new()));

    // Variable compartida que indica qué acción está siendo editada actualmente
    let active_editing: Rc<RefCell<Option<(ActionId, Button)>>> = Rc::new(RefCell::new(None));

    // Agrupar acciones por categoría
    let categories = ["Tabs", "File Management", "Navigation", "Views & Panels"];

    for cat in categories {
        let group = adw::PreferencesGroup::builder()
            .title(cat)
            .build();

        let actions_in_cat: Vec<&ActionMeta> = ALL_ACTIONS.iter().filter(|a| a.category == cat).collect();

        for action in actions_in_cat {
            let row = adw::ActionRow::builder()
                .title(action.name)
                .build();

            let suffix_box = Box::builder()
                .orientation(Orientation::Horizontal)
                .spacing(6)
                .valign(Align::Center)
                .build();

            // Botón de revertir individual si está personalizado
            let btn_undo = Button::builder()
                .icon_name("edit-undo-symbolic")
                .tooltip_text("Restore to default shortcut")
                .css_classes(["flat", "circular"])
                .build();

            let btn_badge = Button::builder()
                .css_classes(["flat", "bubble-shortcut-badge", "tab-pill-btn"])
                .tooltip_text("Click to change this keyboard shortcut")
                .build();

            suffix_box.append(&btn_undo);
            suffix_box.append(&btn_badge);
            row.add_suffix(&suffix_box);

            let a_id = action.id;
            let sm_ref = shortcuts.clone();
            let badge_clone = btn_badge.clone();
            let undo_clone = btn_undo.clone();

            // Función local para refrescar esta fila
            let refresh_this = {
                let sm_ref = sm_ref.clone();
                let badge = badge_clone.clone();
                let undo = undo_clone.clone();
                Rc::new(move || {
                    let cur_sc = sm_ref.borrow().get_shortcut(a_id);
                    badge.set_label(&cur_sc);
                    let is_custom = sm_ref.borrow().is_custom(a_id);
                    undo.set_visible(is_custom);
                    if is_custom {
                        badge.add_css_class("tab-active");
                    } else {
                        badge.remove_css_class("tab-active");
                    }
                })
            };

            refresh_this();
            row_refreshers.borrow_mut().push(refresh_this.clone());

            // Conectar clic para revertir individual
            let sm_undo = shortcuts.clone();
            let ref_undo = refresh_this.clone();
            let cb_undo = on_changed.clone();
            let toast_undo = toast_overlay.clone();
            let a_name = action.name;
            btn_undo.connect_clicked(move |_| {
                {
                    sm_undo.borrow_mut().reset_shortcut(a_id);
                }
                ref_undo();
                cb_undo();
                toast_undo.add_toast(adw::Toast::new(&format!("Shortcut for '{}' reset", a_name)));
            });

            // Conectar clic en badge para comenzar edición
            let editing_ref = active_editing.clone();
            let badge_click = btn_badge.clone();
            btn_badge.connect_clicked(move |_| {
                // Cancelar cualquier edición anterior
                if let Some((_, old_btn)) = editing_ref.borrow().as_ref() {
                    old_btn.remove_css_class("editing-shortcut");
                }
                *editing_ref.borrow_mut() = Some((a_id, badge_click.clone()));
                badge_click.set_label("[ Press a key... ]");
                badge_click.add_css_class("editing-shortcut");
            });

            group.add(&row);
        }

        page.add(&group);
    }

    // Filtrado en vivo de búsqueda
    let page_ref = page.clone();
    search_entry.connect_search_changed(move |entry| {
        let query = entry.text().to_lowercase();
        // Iterar por grupos y filas para filtrar
        let mut group_child = page_ref.first_child();
        while let Some(child) = group_child {
            let next_group = child.next_sibling();
            if let Ok(group) = child.downcast::<adw::PreferencesGroup>() {
                let mut row_child = group.first_child();
                let mut any_visible = false;
                while let Some(r_w) = row_child {
                    let next_row = r_w.next_sibling();
                    if let Ok(r) = r_w.downcast::<adw::ActionRow>() {
                        let title = r.title().to_lowercase();
                        let matches = query.is_empty() || title.contains(&query);
                        r.set_visible(matches);
                        if matches {
                            any_visible = true;
                        }
                    }
                    row_child = next_row;
                }
                group.set_visible(any_visible || query.is_empty());
            }
            group_child = next_group;
        }
    });

    // Controlador de teclado para captura del atajo en modo edición
    let key_controller = EventControllerKey::new();
    let sm_key = shortcuts.clone();
    let editing_key = active_editing.clone();
    let refreshers_key = row_refreshers.clone();
    let cb_key = on_changed.clone();
    let toast_key = toast_overlay.clone();

    key_controller.connect_key_pressed(move |_, key, _, modifier| {
        let edit_target = {
            let mut active_opt = editing_key.borrow_mut();
            active_opt.take()
        };

        if let Some((action_id, btn)) = edit_target {
            btn.remove_css_class("editing-shortcut");

            // Si es Escape, cancela la edición sin cambios
            if key == gtk4::gdk::Key::Escape {
                let refreshers = refreshers_key.borrow().clone();
                for r in refreshers {
                    r();
                }
                toast_key.add_toast(adw::Toast::new("Shortcut editing cancelled"));
                return glib::Propagation::Stop;
            }

            // Convertir la pulsación a formato legible de atajo
            if let Some(combo) = ShortcutsManager::event_to_shortcut(key, modifier) {
                // Intentar asignar el atajo con detección y bloqueo de colisiones
                let result = {
                    let mut sm = sm_key.borrow_mut();
                    sm.set_shortcut(action_id, combo.clone())
                };

                let refreshers = refreshers_key.borrow().clone();
                for r in refreshers {
                    r();
                }

                match result {
                    Ok(_) => {
                        cb_key();
                        toast_key.add_toast(adw::Toast::new(&format!("Shortcut updated to: {}", combo)));
                    }
                    Err(err_msg) => {
                        let toast = adw::Toast::new(&format!("⚠️ {}", err_msg));
                        toast.set_timeout(5);
                        toast_key.add_toast(toast);
                    }
                }
                return glib::Propagation::Stop;
            } else {
                // Modificador solo (ej. Ctrl o Shift apretado solo), mantener en modo edición
                *editing_key.borrow_mut() = Some((action_id, btn));
                return glib::Propagation::Proceed;
            }
        }
        glib::Propagation::Proceed
    });

    window.add_controller(key_controller);

    // Botón restaurar todos los atajos
    let sm_reset = shortcuts;
    let refreshers_reset = row_refreshers;
    let cb_reset = on_changed;
    let toast_reset = toast_overlay.clone();
    btn_reset_all.connect_clicked(move |_| {
        {
            sm_reset.borrow_mut().reset_defaults();
        }
        let refreshers = refreshers_reset.borrow().clone();
        for r in refreshers {
            r();
        }
        cb_reset();
        toast_reset.add_toast(adw::Toast::new("All shortcuts have been reset to defaults"));
    });

    scroll.set_child(Some(&page));
    root.append(&scroll);
    toast_overlay.set_child(Some(&root));
    window.set_content(Some(&toast_overlay));

    window.present();
}
