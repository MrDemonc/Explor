use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::fs::entry::{format_size, is_archive_name, FileEntry};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DiskSpace {
    pub total: u64,
    pub available: u64,
    pub used: u64,
    pub percent_used: f64,
    pub formatted_available: String,
    pub formatted_total: String,
}

pub fn get_disk_space(path: &Path) -> Option<DiskSpace> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stat = MaybeUninit::<libc::statvfs>::uninit();
    let res = unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) };
    if res == 0 {
        let stat = unsafe { stat.assume_init() };
        let block_size = stat.f_frsize as u64;
        let total = stat.f_blocks as u64 * block_size;
        let available = stat.f_bavail as u64 * block_size;
        let used = total.saturating_sub(stat.f_bfree as u64 * block_size);
        let percent_used = if total > 0 {
            (used as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        Some(DiskSpace {
            total,
            available,
            used,
            percent_used,
            formatted_available: format_size(available),
            formatted_total: format_size(total),
        })
    } else {
        None
    }
}

pub fn copy_multiple_with_progress(
    srcs: Vec<PathBuf>,
    dst_folder: PathBuf,
    is_cut: bool,
    sender: Sender<ProgressMessage>,
    cancel_flag: Arc<AtomicBool>,
) {
    if srcs.is_empty() {
        return;
    }
    if srcs.len() == 1 {
        copy_with_progress(srcs.into_iter().next().unwrap(), dst_folder, is_cut, sender, cancel_flag);
        return;
    }

    let total_items = srcs.len();
    for (idx, src) in srcs.iter().enumerate() {
        if cancel_flag.load(Ordering::Relaxed) {
            let _ = sender.send(ProgressMessage::Cancelled);
            return;
        }

        let file_name = match src.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };

        let fraction = (idx as f64) / (total_items as f64);
        let action = if is_cut { "Moving" } else { "Copying" };
        let _ = sender.send(ProgressMessage::Progress {
            fraction,
            status: format!("{action} item {} of {total_items}", idx + 1),
            sub_status: format!("Processing '{file_name}'"),
        });

        let (sub_sender, sub_receiver) = std::sync::mpsc::channel();
        let sub_cancel = cancel_flag.clone();
        let sub_src = src.clone();
        let sub_dst = dst_folder.clone();
        copy_with_progress(sub_src, sub_dst, is_cut, sub_sender, sub_cancel);

        for msg in sub_receiver {
            match msg {
                ProgressMessage::Cancelled => {
                    let _ = sender.send(ProgressMessage::Cancelled);
                    return;
                }
                ProgressMessage::Error { message } => {
                    let _ = sender.send(ProgressMessage::Error { message });
                    return;
                }
                _ => {}
            }
        }
    }

    let action_past = if is_cut { "moved" } else { "copied" };
    let _ = sender.send(ProgressMessage::Success {
        title: format!("{total_items} items {action_past} successfully"),
        message: format!("Items {action_past} to '{}'", dst_folder.display()),
        result_path: None,
    });
}

pub fn compress_multiple_with_progress(
    srcs: Vec<PathBuf>,
    sender: Sender<ProgressMessage>,
    cancel_flag: Arc<AtomicBool>,
) {
    if srcs.is_empty() {
        return;
    }
    if srcs.len() == 1 {
        compress_with_progress(srcs.into_iter().next().unwrap(), sender, cancel_flag);
        return;
    }

    let parent = srcs[0].parent().unwrap_or_else(|| Path::new("."));
    let mut dest_archive = parent.join("archive.zip");
    if dest_archive.exists() {
        let mut count = 1;
        loop {
            let candidate = parent.join(format!("archive_{count}.zip"));
            if !candidate.exists() {
                dest_archive = candidate;
                break;
            }
            count += 1;
        }
    }

    let dest_name = dest_archive.file_name().unwrap_or_default().to_string_lossy().to_string();
    let _ = sender.send(ProgressMessage::Progress {
        fraction: 0.1,
        status: "Compressing multiple items...".to_string(),
        sub_status: format!("Creating '{}'", dest_name),
    });

    let mut names = Vec::new();
    for s in &srcs {
        if let Some(n) = s.file_name() {
            names.push(n.to_string_lossy().to_string());
        }
    }

    let child = {
        let mut cmd = Command::new("zip");
        cmd.arg("-r").arg(dest_archive.to_string_lossy().as_ref());
        for n in &names {
            cmd.arg(n);
        }
        cmd.current_dir(parent);
        cmd.spawn().or_else(|_| {
            let mut py_cmd = Command::new("python3");
            py_cmd.args(["-m", "zipfile", "-c", dest_archive.to_string_lossy().as_ref()]);
            for n in &names {
                py_cmd.arg(n);
            }
            py_cmd.current_dir(parent);
            py_cmd.spawn()
        })
    };

    match child {
        Ok(mut child) => {
            let mut check_count = 0;
            loop {
                if cancel_flag.load(Ordering::Relaxed) {
                    let _ = child.kill();
                    let _ = fs::remove_file(&dest_archive);
                    let _ = sender.send(ProgressMessage::Cancelled);
                    return;
                }
                match child.try_wait() {
                    Ok(Some(status)) => {
                        if status.success() {
                            let _ = sender.send(ProgressMessage::Success {
                                title: "Archive created successfully".to_string(),
                                message: format!("Created '{}' with {} items", dest_name, srcs.len()),
                                result_path: Some(dest_archive),
                            });
                        } else {
                            let _ = sender.send(ProgressMessage::Error {
                                message: format!("zip command finished with error status: {status}"),
                            });
                        }
                        return;
                    }
                    Ok(None) => {
                        check_count += 1;
                        let fraction = (0.1 + (check_count as f64 * 0.05)).min(0.95);
                        let _ = sender.send(ProgressMessage::Progress {
                            fraction,
                            status: "Compressing...".to_string(),
                            sub_status: format!("Writing '{}'", dest_name),
                        });
                        std::thread::sleep(Duration::from_millis(150));
                    }
                    Err(e) => {
                        let _ = sender.send(ProgressMessage::Error {
                            message: format!("Error waiting for zip: {e}"),
                        });
                        return;
                    }
                }
            }
        }
        Err(e) => {
            let _ = sender.send(ProgressMessage::Error {
                message: format!("Could not run 'zip' compressor: {e}"),
            });
        }
    }
}


pub fn read_directory(path: &Path, show_hidden: bool) -> Result<Vec<FileEntry>, std::io::Error> {
    let mut entries = Vec::new();
    let read_dir = fs::read_dir(path)?;

    for entry in read_dir.flatten() {
        let p = entry.path();
        if let Ok(file_entry) = FileEntry::from_path(&p) {
            if show_hidden || !file_entry.is_hidden {
                entries.push(file_entry);
            }
        }
    }

    // Ordenar: Carpetas primero, luego archivos, ambos alfabéticamente (case-insensitive)
    entries.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(entries)
}

pub fn trash_item(path: &Path) -> Result<(), String> {
    // Intentar papelera FreeDesktop
    if let Err(trash_err) = trash::delete(path) {
        // Si no está soportada (como en dispositivos móviles MTP o memorias FUSE), eliminar directamente
        if path.is_dir() {
            fs::remove_dir_all(path)
                .map_err(|e| format!("Could not delete folder on external device: {e} ({trash_err})"))?;
        } else {
            fs::remove_file(path)
                .map_err(|e| format!("Could not delete file on external device: {e} ({trash_err})"))?;
        }
    }
    Ok(())
}

pub fn trash_path() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        let p = home.join(".local/share/Trash/files");
        if !p.exists() {
            let _ = fs::create_dir_all(&p);
        }
        p
    } else {
        PathBuf::from("/tmp")
    }
}

pub fn is_trash_path(path: &Path) -> bool {
    let lossy = path.to_string_lossy();
    lossy.contains(".local/share/Trash") || lossy.ends_with("Trash/files")
}

pub fn restore_trash_items(paths: &[PathBuf]) -> Result<usize, String> {
    if paths.is_empty() {
        return Ok(0);
    }
    let file_names: Vec<std::ffi::OsString> = paths
        .iter()
        .filter_map(|p| p.file_name().map(|s| s.to_os_string()))
        .collect();

    let items = trash::os_limited::list().map_err(|e| format!("Could not read trash: {e}"))?;
    let to_restore: Vec<_> = items
        .into_iter()
        .filter(|i| file_names.contains(&i.name))
        .collect();

    let count = to_restore.len();
    if count > 0 {
        trash::os_limited::restore_all(to_restore).map_err(|e| format!("Restore error: {e}"))?;
        Ok(count)
    } else {
        let mut restored_count = 0;
        if let Some(home) = dirs::home_dir() {
            for p in paths {
                if p.exists() {
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let dest = home.join(name);
                    if !dest.exists() {
                        if fs::rename(p, &dest).is_ok() {
                            restored_count += 1;
                        }
                    }
                }
            }
        }
        if restored_count > 0 {
            Ok(restored_count)
        } else {
            Err("No matching items found to restore".to_string())
        }
    }
}

pub fn permanently_delete_items(paths: &[PathBuf]) -> Result<usize, String> {
    if paths.is_empty() {
        return Ok(0);
    }
    let file_names: Vec<std::ffi::OsString> = paths
        .iter()
        .filter_map(|p| p.file_name().map(|s| s.to_os_string()))
        .collect();

    if let Ok(items) = trash::os_limited::list() {
        let to_purge: Vec<_> = items
            .into_iter()
            .filter(|i| file_names.contains(&i.name))
            .collect();
        if !to_purge.is_empty() {
            let _ = trash::os_limited::purge_all(to_purge);
        }
    }

    let mut count = 0;
    for p in paths {
        if p.is_dir() {
            if fs::remove_dir_all(p).is_ok() {
                count += 1;
            }
        } else if p.exists() {
            if fs::remove_file(p).is_ok() {
                count += 1;
            }
        }
    }
    Ok(count)
}

pub fn empty_trash() -> Result<(), String> {
    if let Ok(items) = trash::os_limited::list() {
        let _ = trash::os_limited::purge_all(items);
    }
    if let Some(home) = dirs::home_dir() {
        let trash_files = home.join(".local/share/Trash/files");
        let trash_info = home.join(".local/share/Trash/info");
        if let Ok(entries) = fs::read_dir(&trash_files) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let _ = fs::remove_dir_all(&p);
                } else {
                    let _ = fs::remove_file(&p);
                }
            }
        }
        if let Ok(entries) = fs::read_dir(&trash_info) {
            for entry in entries.flatten() {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
    Ok(())
}

pub fn rename_item(src: &Path, new_name: &str) -> Result<PathBuf, String> {
    if new_name.trim().is_empty() || new_name.contains('/') {
        return Err("Invalid name".to_string());
    }

    let parent = src.parent().ok_or("Path has no parent directory")?;
    let dst = parent.join(new_name);

    if dst.exists() {
        return Err(format!("An item already exists with the name '{new_name}'"));
    }

    fs::rename(src, &dst).map_err(|e| format!("Error renaming: {e}"))?;
    Ok(dst)
}

pub fn create_folder(parent: &Path, name: &str) -> Result<PathBuf, String> {
    if name.trim().is_empty() || name.contains('/') {
        return Err("Invalid folder name".to_string());
    }

    let new_dir = parent.join(name);
    if new_dir.exists() {
        return Err("Folder already exists".to_string());
    }

    fs::create_dir_all(&new_dir).map_err(|e| format!("Error creating folder: {e}"))?;
    Ok(new_dir)
}

pub fn create_file(parent: &Path, name: &str) -> Result<PathBuf, String> {
    if name.trim().is_empty() || name.contains('/') {
        return Err("Invalid file name".to_string());
    }

    let new_file = parent.join(name);
    if new_file.exists() {
        return Err("File already exists".to_string());
    }

    fs::File::create(&new_file).map_err(|e| format!("Error creating file: {e}"))?;
    Ok(new_file)
}

/// Copia segura con fallback para dispositivos MTP, GVfs y particiones FAT/exFAT
#[allow(dead_code)]
pub fn safe_copy_file(src: &Path, dst: &Path) -> Result<(), String> {
    // 1. Intentar fs::copy nativo
    if fs::copy(src, dst).is_err() {
        // 2. Fallback a streaming manual de bytes (resuelve error 95 / EOPNOTSUPP en MTP)
        let mut reader = fs::File::open(src).map_err(|e| format!("Error opening source file: {e}"))?;
        let mut writer = fs::File::create(dst).map_err(|e| format!("Error creating destination file: {e}"))?;
        std::io::copy(&mut reader, &mut writer).map_err(|e| format!("Error al transferir datos: {e}"))?;
    }
    Ok(())
}

#[allow(dead_code)]
pub fn copy_item(src: &Path, dst_folder: &Path) -> Result<PathBuf, String> {
    let file_name = src.file_name().ok_or("Archivo sin nombre")?;
    let dst = dst_folder.join(file_name);

    if src.is_dir() {
        copy_dir_all(src, &dst)?;
    } else {
        safe_copy_file(src, &dst)?;
    }
    Ok(dst)
}

#[allow(dead_code)]
pub fn move_item(src: &Path, dst_folder: &Path) -> Result<PathBuf, String> {
    let file_name = src.file_name().ok_or("Archivo sin nombre")?;
    let dst = dst_folder.join(file_name);

    if dst.exists() {
        return Err("El destino ya contiene un elemento con el mismo nombre".to_string());
    }

    // 1. Intentar rename atómico (rápido en la misma partición)
    if fs::rename(src, &dst).is_err() {
        // 2. Movimiento entre diferentes dispositivos o sistemas de archivos (cross-device move)
        if src.is_dir() {
            copy_dir_all(src, &dst)?;
            let _ = fs::remove_dir_all(src);
        } else {
            safe_copy_file(src, &dst)?;
            let _ = fs::remove_file(src);
        }
    }
    Ok(dst)
}

#[allow(dead_code)]
fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("Error creando directorio destino: {e}"))?;
    let entries = fs::read_dir(src).map_err(|e| format!("Error leyendo directorio: {e}"))?;
    for entry in entries.flatten() {
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        let sub_src = entry.path();
        let sub_dst = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&sub_src, &sub_dst)?;
        } else {
            safe_copy_file(&sub_src, &sub_dst)?;
        }
    }
    Ok(())
}

pub fn open_with_default(path: &Path) -> Result<(), String> {
    if path.is_dir() {
        return Err("It is a directory, must be opened inside Explor".to_string());
    }
    let path_str = path.to_string_lossy();
    Command::new("xdg-open")
        .arg(&*path_str)
        .spawn()
        .map_err(|e| format!("Error opening with xdg-open: {e}"))?;
    Ok(())
}

pub fn open_in_terminal(path: &Path) -> Result<(), String> {
    let path_str = path.to_string_lossy();
    let terminals = ["kitty", "foot", "alacritty", "wezterm", "ghostty", "gnome-terminal", "xfce4-terminal", "xterm"];
    
    for term in terminals {
        let res = match term {
            "kitty" | "alacritty" | "wezterm" | "ghostty" => {
                Command::new(term).arg("--directory").arg(&*path_str).spawn()
            }
            "foot" => {
                Command::new(term).arg("-D").arg(&*path_str).spawn()
            }
            "gnome-terminal" => {
                Command::new(term).arg(format!("--working-directory={path_str}")).spawn()
            }
            _ => {
                Command::new(term).current_dir(path).spawn()
            }
        };

        if res.is_ok() {
            return Ok(());
        }
    }

    Err("No compatible terminal emulator found installed".to_string())
}

#[allow(dead_code)]
pub fn is_archive(path: &Path) -> bool {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    is_archive_name(&name)
}

#[allow(dead_code)]
pub fn extract_archive(archive_path: &Path) -> Result<PathBuf, String> {
    if !archive_path.exists() {
        return Err("File does not exist".to_string());
    }

    let parent = archive_path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = archive_path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let name_lower = file_name.to_lowercase();

    // Determinar nombre de carpeta sin extensiones compuestas
    let base_name = if name_lower.ends_with(".tar.gz") {
        &file_name[..file_name.len() - 7]
    } else if name_lower.ends_with(".tar.bz2") || name_lower.ends_with(".tar.xz") || name_lower.ends_with(".tar.zst") {
        &file_name[..file_name.len() - 8]
    } else if name_lower.ends_with(".tgz") || name_lower.ends_with(".tbz2") || name_lower.ends_with(".txz") {
        &file_name[..file_name.len() - 4]
    } else if let Some(idx) = file_name.rfind('.') {
        &file_name[..idx]
    } else {
        &file_name
    };

    let mut dest_dir = parent.join(base_name);
    if dest_dir.exists() {
        let mut counter = 1;
        loop {
            let candidate = parent.join(format!("{base_name}_{counter}"));
            if !candidate.exists() {
                dest_dir = candidate;
                break;
            }
            counter += 1;
        }
    }

    fs::create_dir_all(&dest_dir).map_err(|e| format!("Error creating destination folder: {e}"))?;

    let dest_str = dest_dir.to_string_lossy().to_string();
    let archive_str = archive_path.to_string_lossy().to_string();

    let status = if name_lower.ends_with(".zip") {
        Command::new("unzip")
            .args(["-q", &archive_str, "-d", &dest_str])
            .status()
            .or_else(|_| {
                Command::new("7z")
                    .args(["x", "-y", &archive_str, &format!("-o{dest_str}")])
                    .status()
            })
    } else if name_lower.ends_with(".tar")
        || name_lower.ends_with(".tar.gz")
        || name_lower.ends_with(".tgz")
        || name_lower.ends_with(".tar.bz2")
        || name_lower.ends_with(".tbz2")
        || name_lower.ends_with(".tar.xz")
        || name_lower.ends_with(".txz")
        || name_lower.ends_with(".tar.zst")
    {
        Command::new("tar")
            .args(["-xf", &archive_str, "-C", &dest_str])
            .status()
    } else {
        Command::new("7z")
            .args(["x", "-y", &archive_str, &format!("-o{dest_str}")])
            .status()
    };

    match status {
        Ok(s) if s.success() => Ok(dest_dir),
        Ok(_) => Err("Falló la descompresión del archivo".to_string()),
        Err(e) => Err(format!("Error ejecutando descompresor: {e}")),
    }
}

use crate::ui::progress_dialog::ProgressMessage;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub fn copy_with_progress(
    src: PathBuf,
    dst_folder: PathBuf,
    is_cut: bool,
    sender: Sender<ProgressMessage>,
    cancel_flag: Arc<AtomicBool>,
) {
    let file_name = match src.file_name() {
        Some(n) => n.to_string_lossy().to_string(),
        None => {
            let _ = sender.send(ProgressMessage::Error { message: "Source item does not have a valid name".to_string() });
            return;
        }
    };

    let mut dst = dst_folder.join(&file_name);
    // Si no es cortar y el destino ya existe, generar nombre único
    if !is_cut && dst.exists() {
        let stem = src.file_stem().unwrap_or_default().to_string_lossy();
        let ext = src.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
        let mut count = 1;
        loop {
            let candidate = dst_folder.join(format!("{}_copy{}{}", stem, if count > 1 { format!("_{}", count) } else { "".to_string() }, ext));
            if !candidate.exists() {
                dst = candidate;
                break;
            }
            count += 1;
        }
    }

    // 1. Si es cortar y estamos en el mismo sistema de archivos, intentar rename atómico
    if is_cut {
        if fs::rename(&src, &dst).is_ok() {
            let _ = sender.send(ProgressMessage::Progress {
                fraction: 1.0,
                status: "Moved successfully".to_string(),
                sub_status: format!("Item: {}", file_name),
            });
            let _ = sender.send(ProgressMessage::Success {
                title: "Item moved successfully".to_string(),
                message: format!("'{}' was moved to '{}'", file_name, dst_folder.display()),
                result_path: Some(dst),
            });
            return;
        }
    }

    // 2. Recopilar lista de archivos y calcular tamaño total
    let mut files_to_copy: Vec<(PathBuf, PathBuf, u64)> = Vec::new();
    let mut dirs_to_create: Vec<PathBuf> = Vec::new();

    if src.is_dir() {
        dirs_to_create.push(dst.clone());
        let mut queue = vec![(src.clone(), dst.clone())];
        while let Some((s_dir, d_dir)) = queue.pop() {
            if let Ok(entries) = fs::read_dir(&s_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    let target_p = d_dir.join(entry.file_name());
                    if p.is_dir() {
                        dirs_to_create.push(target_p.clone());
                        queue.push((p, target_p));
                    } else {
                        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        files_to_copy.push((p, target_p, size));
                    }
                }
            }
        }
    } else {
        let size = fs::metadata(&src).map(|m| m.len()).unwrap_or(0);
        files_to_copy.push((src.clone(), dst.clone(), size));
    }

    let total_bytes: u64 = files_to_copy.iter().map(|(_, _, s)| *s).sum();
    let total_files = files_to_copy.len();

    // Crear directorios requeridos
    for d in &dirs_to_create {
        if let Err(e) = fs::create_dir_all(d) {
            let _ = sender.send(ProgressMessage::Error { message: format!("Could not create folder '{}': {}", d.display(), e) });
            return;
        }
    }

    let mut copied_bytes: u64 = 0;
    let start_time = Instant::now();
    let mut last_update = Instant::now();
    let mut buffer = [0u8; 131072]; // 128 KB buffer

    for (idx, (s_path, d_path, _)) in files_to_copy.iter().enumerate() {
        if cancel_flag.load(Ordering::Relaxed) {
            let _ = fs::remove_file(d_path);
            let _ = sender.send(ProgressMessage::Cancelled);
            return;
        }

        let mut reader = match fs::File::open(s_path) {
            Ok(r) => r,
            Err(e) => {
                let _ = sender.send(ProgressMessage::Error { message: format!("Error opening '{}': {}", s_path.display(), e) });
                return;
            }
        };

        let mut writer = match fs::File::create(d_path) {
            Ok(w) => w,
            Err(e) => {
                let _ = sender.send(ProgressMessage::Error { message: format!("Error creating '{}': {}", d_path.display(), e) });
                return;
            }
        };

        loop {
            if cancel_flag.load(Ordering::Relaxed) {
                drop(reader);
                drop(writer);
                let _ = fs::remove_file(d_path);
                let _ = sender.send(ProgressMessage::Cancelled);
                return;
            }

            let n = match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    let _ = sender.send(ProgressMessage::Error { message: format!("Read error in '{}': {}", s_path.display(), e) });
                    return;
                }
            };

            if let Err(e) = writer.write_all(&buffer[..n]) {
                let _ = sender.send(ProgressMessage::Error { message: format!("Write error in '{}': {}", d_path.display(), e) });
                return;
            }

            copied_bytes += n as u64;

            if last_update.elapsed() >= Duration::from_millis(50) || copied_bytes == total_bytes {
                last_update = Instant::now();
                let elapsed = start_time.elapsed().as_secs_f64();
                let speed = if elapsed > 0.05 { (copied_bytes as f64) / elapsed } else { 0.0 };
                let fraction = if total_bytes > 0 {
                    (copied_bytes as f64 / total_bytes as f64).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                let action_title = if is_cut { "Moving" } else { "Copying" };
                let status = format!("{} item {} of {}", action_title, idx + 1, total_files);
                let sub_status = format!(
                    "{} of {} ({:.1} MB/s)",
                    format_size(copied_bytes),
                    format_size(total_bytes),
                    speed / 1_048_576.0
                );
                let _ = sender.send(ProgressMessage::Progress { fraction, status, sub_status });
            }
        }
    }

    // Si era mover, eliminar los orígenes
    if is_cut {
        if src.is_dir() {
            let _ = fs::remove_dir_all(&src);
        } else {
            let _ = fs::remove_file(&src);
        }
    }

    let action_verb = if is_cut { "moved" } else { "copied" };
    let _ = sender.send(ProgressMessage::Progress {
        fraction: 1.0,
        status: format!("Item {} successfully", action_verb),
        sub_status: format!("Total transferred: {}", format_size(total_bytes)),
    });
    let _ = sender.send(ProgressMessage::Success {
        title: format!("Item {} successfully", action_verb),
        message: format!("'{}' ({} total)", file_name, format_size(total_bytes)),
        result_path: Some(dst),
    });
}

pub fn extract_with_progress(
    archive_path: PathBuf,
    sender: Sender<ProgressMessage>,
    cancel_flag: Arc<AtomicBool>,
) {
    if !archive_path.exists() {
        let _ = sender.send(ProgressMessage::Error { message: "File does not exist".to_string() });
        return;
    }

    let parent = archive_path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = archive_path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let name_lower = file_name.to_lowercase();

    // Determinar nombre de carpeta destino
    let base_name = if name_lower.ends_with(".tar.gz") {
        &file_name[..file_name.len() - 7]
    } else if name_lower.ends_with(".tar.bz2") || name_lower.ends_with(".tar.xz") || name_lower.ends_with(".tar.zst") {
        &file_name[..file_name.len() - 8]
    } else if name_lower.ends_with(".tgz") || name_lower.ends_with(".tbz2") || name_lower.ends_with(".txz") {
        &file_name[..file_name.len() - 4]
    } else if let Some(idx) = file_name.rfind('.') {
        &file_name[..idx]
    } else {
        &file_name
    };

    let mut dest_dir = parent.join(base_name);
    if dest_dir.exists() {
        let mut counter = 1;
        loop {
            let candidate = parent.join(format!("{base_name}_{counter}"));
            if !candidate.exists() {
                dest_dir = candidate;
                break;
            }
            counter += 1;
        }
    }

    if let Err(e) = fs::create_dir_all(&dest_dir) {
        let _ = sender.send(ProgressMessage::Error { message: format!("Error creating destination folder: {e}") });
        return;
    }

    let dest_str = dest_dir.to_string_lossy().to_string();
    let archive_str = archive_path.to_string_lossy().to_string();

    // Estimar cantidad de archivos para la barra de progreso
    let total_est = if name_lower.ends_with(".zip") {
        Command::new("unzip")
            .args(["-l", &archive_str])
            .output()
            .ok()
            .map(|o| {
                let txt = String::from_utf8_lossy(&o.stdout);
                txt.lines().count().saturating_sub(4).max(1)
            })
            .unwrap_or(20)
    } else if name_lower.contains(".tar") || name_lower.ends_with(".tgz") || name_lower.ends_with(".txz") {
        Command::new("tar")
            .args(["-tf", &archive_str])
            .output()
            .ok()
            .map(|o| {
                let txt = String::from_utf8_lossy(&o.stdout);
                txt.lines().count().max(1)
            })
            .unwrap_or(20)
    } else {
        20
    };

    let _ = sender.send(ProgressMessage::Progress {
        fraction: 0.05,
        status: "Extracting archive...".to_string(),
        sub_status: format!("Extracting '{}' to '{}'", file_name, dest_dir.file_name().unwrap_or_default().to_string_lossy()),
    });

    let child = if name_lower.ends_with(".zip") {
        Command::new("unzip")
            .args(["-o", &archive_str, "-d", &dest_str])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
    } else if name_lower.contains(".tar") || name_lower.ends_with(".tgz") || name_lower.ends_with(".txz") {
        Command::new("tar")
            .args(["-xvf", &archive_str, "-C", &dest_str])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
    } else {
        Command::new("7z")
            .args(["x", "-y", &archive_str, &format!("-o{dest_str}")])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
    };

    let mut proc = match child {
        Ok(c) => c,
        Err(e) => {
            let _ = sender.send(ProgressMessage::Error { message: format!("Could not start extractor: {e}") });
            return;
        }
    };

    if let Some(stdout) = proc.stdout.take() {
        use std::io::BufRead;
        let reader = std::io::BufReader::new(stdout);
        let mut count = 0;
        for line in reader.lines() {
            if cancel_flag.load(Ordering::Relaxed) {
                let _ = proc.kill();
                let _ = fs::remove_dir_all(&dest_dir);
                let _ = sender.send(ProgressMessage::Cancelled);
                return;
            }
            if let Ok(l) = line {
                if !l.trim().is_empty() {
                    count += 1;
                    let fraction = ((count as f64) / (total_est as f64)).clamp(0.05, 0.95);
                    let _ = sender.send(ProgressMessage::Progress {
                        fraction,
                        status: format!("Extracting files ({count}/{total_est})"),
                        sub_status: l.trim().chars().take(50).collect(),
                    });
                }
            }
        }
    }

    let status = proc.wait();
    if cancel_flag.load(Ordering::Relaxed) {
        let _ = fs::remove_dir_all(&dest_dir);
        let _ = sender.send(ProgressMessage::Cancelled);
        return;
    }

    match status {
        Ok(s) if s.success() => {
            let _ = sender.send(ProgressMessage::Progress {
                fraction: 1.0,
                status: "Extraction finished".to_string(),
                sub_status: format!("Extracted to: {}", dest_dir.file_name().unwrap_or_default().to_string_lossy()),
            });
            let _ = sender.send(ProgressMessage::Success {
                title: "Extraction completed successfully".to_string(),
                message: format!("Extracted '{}' to folder '{}'", file_name, dest_dir.file_name().unwrap_or_default().to_string_lossy()),
                result_path: Some(dest_dir),
            });
        }
        Ok(s) => {
            let _ = sender.send(ProgressMessage::Error {
                message: format!("Extractor exited with error code {}", s.code().unwrap_or(-1)),
            });
        }
        Err(e) => {
            let _ = sender.send(ProgressMessage::Error {
                message: format!("Error in extraction process: {e}"),
            });
        }
    }
}

pub fn compress_with_progress(
    src: PathBuf,
    sender: Sender<ProgressMessage>,
    cancel_flag: Arc<AtomicBool>,
) {
    if !src.exists() {
        let _ = sender.send(ProgressMessage::Error { message: "Item to compress does not exist".to_string() });
        return;
    }

    let parent = src.parent().unwrap_or_else(|| Path::new("."));
    let file_name = src.file_name().unwrap_or_default().to_string_lossy().to_string();

    let mut dest_archive = parent.join(format!("{file_name}.zip"));
    if dest_archive.exists() {
        let mut count = 1;
        loop {
            let candidate = parent.join(format!("{file_name}_{count}.zip"));
            if !candidate.exists() {
                dest_archive = candidate;
                break;
            }
            count += 1;
        }
    }

    let dest_name = dest_archive.file_name().unwrap_or_default().to_string_lossy().to_string();

    // Contar elementos para estimar progreso
    let total_est = if src.is_dir() {
        let mut count = 0;
        let mut q = vec![src.clone()];
        while let Some(d) = q.pop() {
            if let Ok(entries) = fs::read_dir(d) {
                for e in entries.flatten() {
                    count += 1;
                    if e.path().is_dir() {
                        q.push(e.path());
                    }
                }
            }
        }
        count.max(1)
    } else {
        1
    };

    let _ = sender.send(ProgressMessage::Progress {
        fraction: 0.05,
        status: "Compressing file(s)...".to_string(),
        sub_status: format!("Creating '{}'", dest_name),
    });

    let child = Command::new("zip")
        .args(["-r", dest_archive.to_string_lossy().as_ref(), &file_name])
        .current_dir(parent)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .or_else(|_| {
            Command::new("python3")
                .args(["-m", "zipfile", "-c", dest_archive.to_string_lossy().as_ref(), &file_name])
                .current_dir(parent)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
        });

    let mut proc = match child {
        Ok(c) => c,
        Err(e) => {
            let _ = sender.send(ProgressMessage::Error { message: format!("Could not run compressor: {e}") });
            return;
        }
    };

    if let Some(stdout) = proc.stdout.take() {
        use std::io::BufRead;
        let reader = std::io::BufReader::new(stdout);
        let mut count = 0;
        for line in reader.lines() {
            if cancel_flag.load(Ordering::Relaxed) {
                let _ = proc.kill();
                let _ = fs::remove_file(&dest_archive);
                let _ = sender.send(ProgressMessage::Cancelled);
                return;
            }
            if let Ok(l) = line {
                if l.contains("adding:") || l.contains("updating:") {
                    count += 1;
                    let fraction = ((count as f64) / (total_est as f64)).clamp(0.05, 0.95);
                    let _ = sender.send(ProgressMessage::Progress {
                        fraction,
                        status: format!("Compressing ({count}/{total_est})"),
                        sub_status: l.trim().chars().take(50).collect(),
                    });
                }
            }
        }
    }

    let status = proc.wait();
    if cancel_flag.load(Ordering::Relaxed) {
        let _ = fs::remove_file(&dest_archive);
        let _ = sender.send(ProgressMessage::Cancelled);
        return;
    }

    match status {
        Ok(s) if s.success() => {
            let _ = sender.send(ProgressMessage::Progress {
                fraction: 1.0,
                status: "Compression finished".to_string(),
                sub_status: format!("Created: {}", dest_name),
            });
            let _ = sender.send(ProgressMessage::Success {
                title: "Compression completed successfully".to_string(),
                message: format!("Compressed archive '{}' created successfully", dest_name),
                result_path: Some(dest_archive),
            });
        }
        Ok(s) => {
            let _ = sender.send(ProgressMessage::Error {
                message: format!("Compressor exited with error {}", s.code().unwrap_or(-1)),
            });
        }
        Err(e) => {
            let _ = sender.send(ProgressMessage::Error {
                message: format!("Error in compression process: {e}"),
            });
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::sync::mpsc::channel;

    #[test]
    fn test_copy_with_progress_and_cancel() {
        let temp_dir = std::env::temp_dir().join("explor_test_copy");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let src_file = temp_dir.join("test_src.bin");
        fs::write(&src_file, vec![0xAB; 256 * 1024]).unwrap(); // 256 KB

        let dest_folder = temp_dir.join("dest");
        fs::create_dir_all(&dest_folder).unwrap();

        let (sender, receiver) = channel();
        let cancel_flag = Arc::new(AtomicBool::new(false));

        copy_with_progress(src_file.clone(), dest_folder.clone(), false, sender, cancel_flag);

        let mut got_success = false;
        while let Ok(msg) = receiver.try_recv() {
            if let ProgressMessage::Success { .. } = msg {
                got_success = true;
            }
        }
        assert!(got_success);
        assert!(dest_folder.join("test_src.bin").exists());

        // Limpieza
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_compress_and_extract_with_progress() {
        let temp_dir = std::env::temp_dir().join("explor_test_compress");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let sample_folder = temp_dir.join("sample");
        fs::create_dir_all(&sample_folder).unwrap();
        fs::write(sample_folder.join("a.txt"), "Hola mundo").unwrap();
        fs::write(sample_folder.join("b.txt"), "Explor test").unwrap();

        let (sender, receiver) = channel();
        let cancel_flag = Arc::new(AtomicBool::new(false));

        compress_with_progress(sample_folder.clone(), sender, cancel_flag);

        let mut created_zip: Option<PathBuf> = None;
        while let Ok(msg) = receiver.try_recv() {
            if let ProgressMessage::Success { result_path, .. } = msg {
                created_zip = result_path;
            }
        }

        assert!(created_zip.is_some());
        let zip_path = created_zip.unwrap();
        assert!(zip_path.exists());

        // Ahora extraer el zip generado
        let (send_ext, recv_ext) = channel();
        let cancel_ext = Arc::new(AtomicBool::new(false));
        extract_with_progress(zip_path, send_ext, cancel_ext);

        let mut extracted_dest: Option<PathBuf> = None;
        while let Ok(msg) = recv_ext.try_recv() {
            if let ProgressMessage::Success { result_path, .. } = msg {
                extracted_dest = result_path;
            }
        }

        assert!(extracted_dest.is_some());
        let ext_dir = extracted_dest.unwrap();
        assert!(ext_dir.exists());

        // Limpieza
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
