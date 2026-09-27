use gtk4::prelude::*;
use gtk4::{Align, Box, Button, EventControllerKey, HeaderBar, Image, Label, Orientation, ProgressBar};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Debug)]
pub enum ProgressMessage {
    Progress {
        fraction: f64,
        status: String,
        sub_status: String,
    },
    Success {
        title: String,
        message: String,
        result_path: Option<PathBuf>,
    },
    Error {
        message: String,
    },
    Cancelled,
}

pub fn send_system_notification(title: &str, body: &str, is_error: bool) {
    let icon = if is_error {
        "dialog-error-symbolic"
    } else {
        "document-save-symbolic"
    };
    let _ = Command::new("notify-send")
        .args(["-a", "Explor", "-i", icon, title, body])
        .spawn();
}

#[allow(dead_code)]
pub struct ProgressDialog {
    window: adw::Window,
    btn_done: Button,
    last_result_path: Rc<RefCell<Option<PathBuf>>>,
}

#[allow(dead_code)]
impl ProgressDialog {
    pub fn new<W: IsA<gtk4::Window>>(
        parent: &W,
        initial_title: &str,
        icon_name: &str,
    ) -> (Self, Sender<ProgressMessage>, Arc<AtomicBool>) {
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let (sender, receiver): (Sender<ProgressMessage>, Receiver<ProgressMessage>) = channel();
        let last_result_path = Rc::new(RefCell::new(None));

        let window = adw::Window::builder()
            .transient_for(parent)
            .modal(true)
            .title(initial_title)
            .default_width(460)
            .default_height(230)
            .resizable(false)
            .build();

        let root = Box::builder()
            .orientation(Orientation::Vertical)
            .build();

        let header = HeaderBar::builder()
            .show_title_buttons(false)
            .build();
        root.append(&header);

        let content_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(14)
            .margin_start(24)
            .margin_end(24)
            .margin_top(16)
            .margin_bottom(20)
            .build();

        // Cabecera con Icono y Títulos
        let top_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(16)
            .valign(Align::Center)
            .build();

        let icon = Image::builder()
            .icon_name(icon_name)
            .pixel_size(36)
            .valign(Align::Center)
            .build();

        let text_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(3)
            .hexpand(true)
            .build();

        let title_label = Label::builder()
            .label(initial_title)
            .halign(Align::Start)
            .css_classes(["heading"])
            .wrap(true)
            .build();

        let subtitle_label = Label::builder()
            .label("Starting operation...")
            .halign(Align::Start)
            .css_classes(["caption", "dim-label"])
            .wrap(true)
            .build();

        text_box.append(&title_label);
        text_box.append(&subtitle_label);
        top_box.append(&icon);
        top_box.append(&text_box);
        content_box.append(&top_box);

        // Barra de progreso
        let progress_bar = ProgressBar::builder()
            .fraction(0.0)
            .show_text(true)
            .text("0%")
            .css_classes(["progress-bar-custom"])
            .build();
        content_box.append(&progress_bar);

        // Etiqueta de detalle (velocidad, bytes, etc.)
        let detail_label = Label::builder()
            .label("Preparing data...")
            .halign(Align::Start)
            .css_classes(["caption", "dim-label"])
            .build();
        content_box.append(&detail_label);

        // Botones inferiores (Cancelar / Visto)
        let btn_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(10)
            .halign(Align::End)
            .margin_top(8)
            .build();

        let btn_cancel = Button::builder()
            .label("Cancel")
            .icon_name("process-stop-symbolic")
            .css_classes(["destructive-action"])
            .build();

        let btn_done = Button::builder()
            .label("✓ Dismiss")
            .css_classes(["suggested-action"])
            .visible(false)
            .build();

        btn_box.append(&btn_cancel);
        btn_box.append(&btn_done);
        content_box.append(&btn_box);

        root.append(&content_box);
        window.set_content(Some(&root));

        // Manejador de cancelación
        let cancel_flag_clone = cancel_flag.clone();
        let btn_cancel_clone = btn_cancel.clone();
        let subtitle_clone = subtitle_label.clone();
        btn_cancel.connect_clicked(move |_| {
            cancel_flag_clone.store(true, Ordering::Relaxed);
            btn_cancel_clone.set_sensitive(false);
            btn_cancel_clone.set_label("Cancelling...");
            subtitle_clone.set_text("Stopping operation and cleaning up...");
        });

        // Manejador de teclado: Enter o Esc activan Visto si ya terminó
        let key_ctrl = EventControllerKey::new();
        let btn_done_key = btn_done.clone();
        let btn_cancel_key = btn_cancel.clone();
        key_ctrl.connect_key_pressed(move |_, key, _, _| {
            if btn_done_key.is_visible() {
                if key == gtk4::gdk::Key::Return || key == gtk4::gdk::Key::KP_Enter || key == gtk4::gdk::Key::Escape {
                    btn_done_key.emit_clicked();
                    return glib::Propagation::Stop;
                }
            } else if key == gtk4::gdk::Key::Escape {
                if btn_cancel_key.is_sensitive() {
                    btn_cancel_key.emit_clicked();
                    return glib::Propagation::Stop;
                }
            }
            glib::Propagation::Proceed
        });
        window.add_controller(key_ctrl);

        // Suscripción al canal de mensajes mediante temporizador de la interfaz
        let icon_recv = icon;
        let title_recv = title_label;
        let subtitle_recv = subtitle_label;
        let progress_recv = progress_bar;
        let detail_recv = detail_label;
        let btn_cancel_recv = btn_cancel;
        let btn_done_recv = btn_done.clone();
        let path_recv = last_result_path.clone();

        glib::timeout_add_local(Duration::from_millis(25), move || {
            let mut stop_timer = false;
            while let Ok(msg) = receiver.try_recv() {
                match msg {
                    ProgressMessage::Progress { fraction, status, sub_status } => {
                        let clamped = fraction.clamp(0.0, 1.0);
                        progress_recv.set_fraction(clamped);
                        progress_recv.set_text(Some(&format!("{:.0}%", clamped * 100.0)));
                        if !status.is_empty() {
                            title_recv.set_text(&status);
                        }
                        if !sub_status.is_empty() {
                            detail_recv.set_text(&sub_status);
                        }
                    }
                    ProgressMessage::Success { title, message, result_path } => {
                        progress_recv.set_fraction(1.0);
                        progress_recv.set_text(Some("100%"));
                        icon_recv.set_icon_name(Some("object-select-symbolic"));
                        icon_recv.add_css_class("success-icon");
                        title_recv.set_text(&title);
                        subtitle_recv.set_text(&message);
                        detail_recv.set_text("Operation completed successfully.");

                        *path_recv.borrow_mut() = result_path;

                        btn_cancel_recv.set_visible(false);
                        btn_done_recv.set_visible(true);
                        btn_done_recv.grab_focus();

                        send_system_notification(&title, &message, false);
                        stop_timer = true;
                    }
                    ProgressMessage::Error { message } => {
                        icon_recv.set_icon_name(Some("dialog-error-symbolic"));
                        icon_recv.add_css_class("error-icon");
                        title_recv.set_text("Operation error");
                        subtitle_recv.set_text(&message);
                        detail_recv.set_text("Could not complete the requested task.");

                        btn_cancel_recv.set_visible(false);
                        btn_done_recv.set_visible(true);
                        btn_done_recv.grab_focus();

                        send_system_notification("Operation error", &message, true);
                        stop_timer = true;
                    }
                    ProgressMessage::Cancelled => {
                        icon_recv.set_icon_name(Some("dialog-warning-symbolic"));
                        icon_recv.add_css_class("warning-icon");
                        title_recv.set_text("Operation cancelled");
                        subtitle_recv.set_text("Operation was stopped by the user.");
                        detail_recv.set_text("Incomplete changes have been discarded.");

                        btn_cancel_recv.set_visible(false);
                        btn_done_recv.set_visible(true);
                        btn_done_recv.grab_focus();

                        send_system_notification("Operation cancelled", "The operation was interrupted.", true);
                        stop_timer = true;
                    }
                }
            }
            if stop_timer {
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });

        let dialog = Self {
            window,
            btn_done,
            last_result_path,
        };

        (dialog, sender, cancel_flag)
    }

    pub fn present(&self) {
        self.window.present();
    }

    pub fn connect_finished<F>(&self, on_finished: F)
    where
        F: Fn(bool, Option<PathBuf>) + 'static,
    {
        let win = self.window.clone();
        let path_ref = self.last_result_path.clone();
        self.btn_done.connect_clicked(move |_| {
            let res_path = path_ref.borrow().clone();
            win.close();
            on_finished(true, res_path);
        });
    }
}
