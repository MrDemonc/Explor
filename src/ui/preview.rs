use std::fs;
use std::io::Read;
use std::path::Path;
use gtk4::prelude::*;
use gtk4::{Align, Box, Label, Orientation, Picture, ScrolledWindow, TextView, Video};
use crate::fs::entry::FileEntry;

#[derive(Clone)]
pub struct PreviewPane {
    container: Box,
    status_label: Label,
    content_box: Box,
    icon_image: gtk4::Image,
    title_label: Label,
    meta_box: Box,
    text_view: TextView,
    text_scroll: ScrolledWindow,
    picture: Picture,
    video: Video,
}

impl PreviewPane {
    pub fn new() -> Self {
        let container = Box::builder()
            .orientation(Orientation::Vertical)
            .width_request(280)
            .css_classes(["preview-pane"])
            .build();

        let status_label = Label::builder()
            .label("Select a file to view details")
            .wrap(true)
            .valign(Align::Center)
            .vexpand(true)
            .css_classes(["dim-label", "title-4"])
            .build();

        let content_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(12)
            .margin_top(16)
            .margin_bottom(16)
            .margin_start(16)
            .margin_end(16)
            .visible(false)
            .build();

        // Icono y título
        let icon_image = gtk4::Image::builder()
            .icon_name("text-x-generic")
            .pixel_size(42)
            .halign(Align::Center)
            .build();

        let title_label = Label::builder()
            .label("")
            .wrap(true)
            .wrap_mode(gtk4::pango::WrapMode::WordChar)
            .halign(Align::Center)
            .css_classes(["title-3", "file-title"])
            .selectable(true)
            .build();

        // Metadatos
        let meta_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(6)
            .css_classes(["card", "preview-meta-card"])
            .build();

        // Visor de texto
        let text_view = TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .monospace(true)
            .wrap_mode(gtk4::WrapMode::WordChar)
            .left_margin(10)
            .right_margin(10)
            .top_margin(10)
            .bottom_margin(10)
            .css_classes(["preview-text-view"])
            .build();

        let text_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .min_content_height(180)
            .vexpand(true)
            .child(&text_view)
            .css_classes(["preview-scroll", "card"])
            .build();

        // Visor de imagen
        let picture = Picture::builder()
            .can_shrink(true)
            .height_request(200)
            .css_classes(["preview-picture", "card"])
            .build();

        // Visor multimedia (Video / Audio con controles integrados)
        let video = Video::builder()
            .autoplay(false)
            .loop_(false)
            .height_request(200)
            .css_classes(["preview-video", "card"])
            .build();

        content_box.append(&icon_image);
        content_box.append(&title_label);
        content_box.append(&picture);
        content_box.append(&video);
        content_box.append(&meta_box);
        content_box.append(&text_scroll);

        container.append(&status_label);
        container.append(&content_box);

        Self {
            container,
            status_label,
            content_box,
            icon_image,
            title_label,
            meta_box,
            text_view,
            text_scroll,
            picture,
            video,
        }
    }

    pub fn widget(&self) -> &Box {
        &self.container
    }

    pub fn clear(&self) {
        self.video.set_file(None::<&gio::File>);
        self.status_label.set_visible(true);
        self.content_box.set_visible(false);
    }

    pub fn update_multi(&self, entries: &[FileEntry]) {
        self.video.set_file(None::<&gio::File>);
        self.video.set_visible(false);
        self.picture.set_visible(false);
        self.status_label.set_visible(false);
        self.content_box.set_visible(true);

        self.icon_image.set_icon_name(Some("edit-select-all-symbolic"));
        self.title_label.set_text(&format!("{} items selected", entries.len()));

        while let Some(child) = self.meta_box.first_child() {
            self.meta_box.remove(&child);
        }

        let total_size: u64 = entries.iter().map(|e| e.size).sum();
        let folder_count = entries.iter().filter(|e| e.is_dir).count();
        let file_count = entries.len() - folder_count;

        self.meta_box.append(&create_meta_row("Total Size:", &crate::fs::entry::format_size(total_size)));
        self.meta_box.append(&create_meta_row("Folders:", &folder_count.to_string()));
        self.meta_box.append(&create_meta_row("Files:", &file_count.to_string()));

        let mut summary = String::new();
        for e in entries.iter().take(25) {
            let prefix = if e.is_dir { "📁 " } else { "📄 " };
            summary.push_str(&format!("{prefix}{}\n", e.name));
        }
        if entries.len() > 25 {
            summary.push_str(&format!("... and {} more\n", entries.len() - 25));
        }
        self.text_scroll.set_visible(true);
        self.text_view.buffer().set_text(&summary);
    }

    pub fn update(&self, entry: &FileEntry) {
        self.status_label.set_visible(false);
        self.content_box.set_visible(true);

        self.icon_image.set_icon_name(Some(entry.icon_name));
        self.title_label.set_text(&entry.name);

        // Limpiar metadatos previos
        while let Some(child) = self.meta_box.first_child() {
            self.meta_box.remove(&child);
        }

        // Agregar filas de metadatos
        self.meta_box.append(&create_meta_row("Size:", &entry.formatted_size));
        self.meta_box.append(&create_meta_row("Modified:", &entry.formatted_date));
        self.meta_box.append(&create_meta_row("Permissions:", &entry.permissions));

        let type_desc = if entry.is_dir {
            "Folder"
        } else if entry.is_symlink {
            "Symbolic Link"
        } else if entry.is_executable {
            "Binary Executable"
        } else {
            "File"
        };
        self.meta_box.append(&create_meta_row("Type:", type_desc));

        // Vista previa según el tipo
        let ext = entry.path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let is_image = matches!(
            ext.as_str(),
            "png" | "jpg" | "jpeg" | "webp" | "svg" | "gif" | "bmp" | "ico" | "avif" | "tiff"
        );

        let is_multimedia = matches!(
            ext.as_str(),
            "mp4" | "mkv" | "avi" | "webm" | "mov" | "flv" | "wmv" | "m4v" | "3gp"
            | "mp3" | "flac" | "ogg" | "wav" | "m4a" | "aac" | "opus" | "wma"
        );

        if is_image && !entry.is_dir {
            self.video.set_file(None::<&gio::File>);
            self.video.set_visible(false);
            let file = gio::File::for_path(&entry.path);
            self.picture.set_file(Some(&file));
            self.picture.set_visible(true);
            self.text_scroll.set_visible(false);
        } else if is_multimedia && !entry.is_dir {
            self.picture.set_visible(false);
            self.text_scroll.set_visible(false);
            let file = gio::File::for_path(&entry.path);
            self.video.set_file(Some(&file));
            self.video.set_visible(true);
        } else if !entry.is_dir && is_text_file(&entry.path, &ext) {
            self.video.set_file(None::<&gio::File>);
            self.video.set_visible(false);
            if let Ok(mut file) = fs::File::open(&entry.path) {
                let mut buffer = Vec::new();
                // Leer hasta 32 KB para mantener la UI ultra rápida
                let _ = file.by_ref().take(32768).read_to_end(&mut buffer);

                // Si contiene bytes nulos, es un archivo binario
                if buffer.contains(&0) {
                    self.picture.set_visible(false);
                    self.text_scroll.set_visible(false);
                } else {
                    self.picture.set_visible(false);
                    self.text_scroll.set_visible(true);
                    let text = String::from_utf8_lossy(&buffer);
                    let clean_text = text.replace('\0', "");
                    self.text_view.buffer().set_text(&clean_text);
                }
            } else {
                self.picture.set_visible(false);
                self.text_scroll.set_visible(false);
            }
        } else if entry.is_dir {
            self.video.set_file(None::<&gio::File>);
            self.video.set_visible(false);
            self.picture.set_visible(false);
            self.text_scroll.set_visible(true);
            
            // Mostrar una vista previa rápida de los primeros elementos de la carpeta
            let mut summary = String::new();
            if let Ok(rd) = fs::read_dir(&entry.path) {
                let mut items: Vec<_> = rd.flatten().take(15).collect();
                items.sort_by_key(|e| e.file_name());
                for item in items {
                    let name = item.file_name().to_string_lossy().to_string();
                    let is_d = item.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    let prefix = if is_d { "📁 " } else { "📄 " };
                    summary.push_str(&format!("{prefix}{name}\n"));
                }
            }
            if summary.is_empty() {
                summary = "Empty folder".to_string();
            }
            let clean_summary = summary.replace('\0', "");
            self.text_view.buffer().set_text(&clean_summary);
        } else {
            self.video.set_file(None::<&gio::File>);
            self.video.set_visible(false);
            self.picture.set_visible(false);
            self.text_scroll.set_visible(false);
        }
    }
}

fn create_meta_row(label_text: &str, value_text: &str) -> Box {
    let row = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .build();

    let label = Label::builder()
        .label(label_text)
        .css_classes(["dim-label", "caption"])
        .build();

    let value = Label::builder()
        .label(value_text)
        .hexpand(true)
        .halign(Align::End)
        .css_classes(["caption", "numeric"])
        .selectable(true)
        .build();

    row.append(&label);
    row.append(&value);
    row
}

fn is_text_file(path: &Path, ext: &str) -> bool {
    let known_binary_exts = [
        "o", "a", "so", "bin", "exe", "dll", "dylib", "class", "pyc", "pyo",
        "iso", "img", "tar", "gz", "xz", "bz2", "7z", "zip", "rar", "zst",
        "pdf", "docx", "xlsx", "pptx", "sqlite", "db", "wasm"
    ];

    if known_binary_exts.contains(&ext) {
        return false;
    }

    let known_text_exts = [
        "txt", "md", "rs", "py", "js", "ts", "json", "toml", "yaml", "yml",
        "html", "css", "scss", "c", "cpp", "h", "hpp", "go", "sh", "bash",
        "zsh", "fish", "lua", "conf", "ini", "log", "xml", "sql", "env",
        "desktop", "service", "diff", "patch", "vue", "svelte", "jsx", "tsx"
    ];

    if known_text_exts.contains(&ext) {
        return true;
    }

    // Si no tiene extensión o es desconocida, revisar si los primeros 2048 bytes no contienen bytes nulos
    if let Ok(mut f) = fs::File::open(path) {
        let mut buf = [0u8; 2048];
        if let Ok(n) = f.read(&mut buf) {
            return n > 0 && !buf[..n].contains(&0);
        }
    }
    false
}
