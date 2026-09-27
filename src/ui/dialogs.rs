use libadwaita as adw;
use libadwaita::prelude::*;
use gtk4::{Align, Box, Button, Entry, EventControllerKey, Label, Orientation, Window};
use gtk4::gdk::Key;

pub fn prompt_input<W, F>(
    parent: &W,
    title_text: &str,
    placeholder: &str,
    default_value: &str,
    confirm_label_text: &str,
    on_confirm: F,
) where
    W: IsA<gtk4::Window>,
    F: Fn(String) + 'static,
{
    let dialog = Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(title_text)
        .default_width(380)
        .resizable(false)
        .decorated(true)
        .build();

    let root_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(16)
        .margin_top(20)
        .margin_bottom(20)
        .margin_start(20)
        .margin_end(20)
        .build();

    let title = Label::builder()
        .label(title_text)
        .halign(Align::Start)
        .css_classes(["title-3", "heading"])
        .build();

    let entry = Entry::builder()
        .placeholder_text(placeholder)
        .text(default_value)
        .hexpand(true)
        .build();

    let btn_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .halign(Align::End)
        .build();

    let cancel_btn = Button::builder()
        .label("Cancel")
        .build();

    let confirm_btn = Button::builder()
        .label(confirm_label_text)
        .css_classes(["suggested-action"])
        .build();

    btn_box.append(&cancel_btn);
    btn_box.append(&confirm_btn);

    root_box.append(&title);
    root_box.append(&entry);
    root_box.append(&btn_box);

    dialog.set_child(Some(&root_box));

    // Cancelar
    let dlg_cancel = dialog.clone();
    cancel_btn.connect_clicked(move |_| {
        dlg_cancel.close();
    });

    // Confirmar
    let dlg_confirm = dialog.clone();
    let entry_clone = entry.clone();
    let on_conf = std::rc::Rc::new(on_confirm);
    let on_conf_clone = on_conf.clone();

    let do_confirm = move || {
        let text = entry_clone.text().to_string();
        if !text.trim().is_empty() {
            on_conf_clone(text);
            dlg_confirm.close();
        }
    };

    let do_c1 = do_confirm.clone();
    confirm_btn.connect_clicked(move |_| {
        do_c1();
    });

    let do_c2 = do_confirm.clone();
    entry.connect_activate(move |_| {
        do_c2();
    });

    // Control de teclas (Escape para cerrar)
    let key_controller = EventControllerKey::new();
    let dlg_key = dialog.clone();
    key_controller.connect_key_pressed(move |_, key, _, _| {
        if key == Key::Escape {
            dlg_key.close();
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    dialog.add_controller(key_controller);

    dialog.present();
    entry.grab_focus();
    entry.select_region(0, -1);
}

pub fn confirm_delete<W, F>(parent: &W, item_name: &str, on_confirm: F)
where
    W: IsA<gtk4::Widget>,
    F: Fn() + 'static,
{
    let alert = adw::AlertDialog::builder()
        .heading("Move to Trash?")
        .body(format!("Do you want to move \"{item_name}\" to trash?"))
        .build();

    alert.add_response("cancel", "Cancel");
    alert.add_response("delete", "Move to Trash");
    alert.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
    alert.set_default_response(Some("delete"));
    alert.set_close_response("cancel");

    alert.choose(Some(parent), None::<&gio::Cancellable>, move |response| {
        if response == "delete" {
            on_confirm();
        }
    });
}

pub fn confirm_permanent_delete<W, F>(parent: &W, item_name: &str, on_confirm: F)
where
    W: IsA<gtk4::Widget>,
    F: Fn() + 'static,
{
    let alert = adw::AlertDialog::builder()
        .heading("Delete Permanently?")
        .body(format!("Are you sure you want to permanently delete \"{item_name}\"? This action cannot be undone."))
        .build();

    alert.add_response("cancel", "Cancel");
    alert.add_response("delete", "Delete Permanently");
    alert.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
    alert.set_default_response(Some("delete"));
    alert.set_close_response("cancel");

    alert.choose(Some(parent), None::<&gio::Cancellable>, move |response| {
        if response == "delete" {
            on_confirm();
        }
    });
}

pub fn confirm_empty_trash<W, F>(parent: &W, on_confirm: F)
where
    W: IsA<gtk4::Widget>,
    F: Fn() + 'static,
{
    let alert = adw::AlertDialog::builder()
        .heading("Empty Trash?")
        .body("Are you sure you want to permanently delete all items from the Trash? This action cannot be undone.")
        .build();

    alert.add_response("cancel", "Cancel");
    alert.add_response("empty", "Empty Trash");
    alert.set_response_appearance("empty", adw::ResponseAppearance::Destructive);
    alert.set_default_response(Some("empty"));
    alert.set_close_response("cancel");

    alert.choose(Some(parent), None::<&gio::Cancellable>, move |response| {
        if response == "empty" {
            on_confirm();
        }
    });
}
