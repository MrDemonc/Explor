use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use chrono::{DateTime, Local};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub is_hidden: bool,
    pub is_archive: bool,
    pub size: u64,
    pub formatted_size: String,
    pub modified: Option<SystemTime>,
    pub formatted_date: String,
    pub permissions: String,
    pub icon_name: &'static str,
    pub is_executable: bool,
}

impl FileEntry {
    pub fn from_path(path: &Path) -> Result<Self, std::io::Error> {
        let symlink_metadata = fs::symlink_metadata(path)?;
        let is_symlink = symlink_metadata.file_type().is_symlink();
        
        let metadata = fs::metadata(path).unwrap_or_else(|_| symlink_metadata.clone());
        let mut is_dir = metadata.is_dir() || path.is_dir();

        // Si es enlace simbólico, verificar si apunta a un directorio
        if is_symlink && !is_dir {
            if let Ok(target) = fs::read_link(path) {
                let resolved = if target.is_relative() {
                    path.parent().map(|p| p.join(&target)).unwrap_or(target)
                } else {
                    target
                };
                if resolved.is_dir() {
                    is_dir = true;
                }
            }
        }
        
        let file_name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        
        let is_hidden = file_name.starts_with('.');
        let size = if is_dir { 0 } else { metadata.len() };
        
        let formatted_size = if is_dir {
            match fs::read_dir(path) {
                Ok(rd) => {
                    let count = rd.count();
                    if count == 1 {
                        "1 item".to_string()
                    } else {
                        format!("{count} items")
                    }
                }
                Err(_) => "—".to_string(),
            }
        } else {
            format_size(size)
        };

        let modified = metadata.modified().ok();
        let formatted_date = modified
            .map(|st| {
                let dt: DateTime<Local> = st.into();
                dt.format("%Y-%m-%d %H:%M").to_string()
            })
            .unwrap_or_else(|| "—".to_string());

        let mode = metadata.permissions().mode();
        let permissions = format_mode(mode, is_dir);
        let is_executable = !is_dir && (mode & 0o111 != 0);
        let is_archive = !is_dir && is_archive_name(&file_name);

        let icon_name = determine_icon(&file_name, is_dir, is_symlink, is_executable, is_archive);

        Ok(Self {
            name: file_name,
            path: path.to_path_buf(),
            is_dir,
            is_symlink,
            is_hidden,
            is_archive,
            size,
            formatted_size,
            modified,
            formatted_date,
            permissions,
            icon_name,
            is_executable,
        })
    }
}

pub fn is_archive_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".zip")
        || lower.ends_with(".tar.gz")
        || lower.ends_with(".tgz")
        || lower.ends_with(".tar.bz2")
        || lower.ends_with(".tbz2")
        || lower.ends_with(".tar.xz")
        || lower.ends_with(".txz")
        || lower.ends_with(".tar.zst")
        || lower.ends_with(".tar")
        || lower.ends_with(".7z")
        || lower.ends_with(".rar")
}

pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes < KB {
        format!("{bytes} B")
    } else if bytes < MB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes < TB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    }
}

fn format_mode(mode: u32, is_dir: bool) -> String {
    let mut s = String::with_capacity(10);
    s.push(if is_dir { 'd' } else { '-' });
    let flags = [
        (0o400, 'r'), (0o200, 'w'), (0o100, 'x'),
        (0o040, 'r'), (0o020, 'w'), (0o010, 'x'),
        (0o004, 'r'), (0o002, 'w'), (0o001, 'x'),
    ];
    for (flag, ch) in flags {
        if mode & flag != 0 {
            s.push(ch);
        } else {
            s.push('-');
        }
    }
    s
}

fn determine_icon(name: &str, is_dir: bool, _is_symlink: bool, is_executable: bool, is_archive: bool) -> &'static str {
    if is_dir {
        let lower = name.to_lowercase();
        match lower.as_str() {
            "downloads" | "descargas" => "folder-download",
            "documents" | "documentos" => "folder-documents",
            "pictures" | "imágenes" | "imagenes" | "fotos" => "folder-pictures",
            "music" | "música" | "musica" => "folder-music",
            "videos" | "vídeos" => "folder-videos",
            "desktop" | "escritorio" => "user-desktop",
            "templates" | "plantillas" => "folder-templates",
            "public" | "público" | "publico" => "folder-publicshare",
            ".git" => "folder",
            _ => "folder",
        }
    } else if is_archive {
        "package-x-generic"
    } else {
        let ext = Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "bmp" | "ico" | "avif" | "tiff" | "heic" => "image-x-generic",
            "mp4" | "mkv" | "avi" | "webm" | "mov" | "flv" | "wmv" | "m4v" | "3gp" => "video-x-generic",
            "mp3" | "flac" | "ogg" | "wav" | "m4a" | "aac" | "opus" | "wma" | "mid" | "midi" => "audio-x-generic",
            "pdf" => "x-office-document",
            "doc" | "docx" | "odt" | "rtf" | "pages" | "epub" | "mobi" => "x-office-document",
            "xls" | "xlsx" | "ods" | "csv" | "tsv" | "numbers" => "x-office-spreadsheet",
            "ppt" | "pptx" | "odp" | "keynote" => "x-office-presentation",
            "ttf" | "otf" | "woff" | "woff2" | "eot" => "font-x-generic",
            "iso" | "img" | "dmg" | "bin" | "cue" => "media-optical",
            "pem" | "crt" | "cer" | "key" | "pub" | "gpg" | "asc" => "application-certificate",
            "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "c" | "cpp" | "cc" | "h" | "hpp" | "go" | "sh" | "bash" | "zsh"
            | "fish" | "lua" | "html" | "css" | "scss" | "json" | "toml" | "yaml" | "yml" | "xml" | "sql" | "php" | "java" | "kt" | "swift" => {
                "text-x-script"
            }
            "txt" | "md" | "markdown" | "log" | "rst" | "ini" | "conf" | "cfg" | "env" => "text-x-generic",
            _ if is_executable => "application-x-executable",
            _ => "text-x-generic",
        }
    }
}
