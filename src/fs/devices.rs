use std::collections::HashSet;
use std::path::{Path, PathBuf};
use gio::prelude::*;
use crate::fs::operations::{get_disk_space, DiskSpace};

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct DeviceInfo {
    pub name: String,
    pub path: Option<PathBuf>,
    pub icon_name: &'static str,
    pub is_mounted: bool,
    pub volume: Option<gio::Volume>,
    pub disk_space: Option<DiskSpace>,
}

pub fn get_storage_devices() -> Vec<DeviceInfo> {
    let mut devices = Vec::new();
    let mut seen_paths: HashSet<PathBuf> = HashSet::new();
    let mut seen_names: HashSet<String> = HashSet::new();

    // 1. Almacenamiento local principal (Raíz / Sistema)
    let root_path = PathBuf::from("/");
    if let Some(space) = get_disk_space(&root_path) {
        devices.push(DeviceInfo {
            name: "Main System".to_string(),
            path: Some(root_path.clone()),
            icon_name: "drive-harddisk-symbolic",
            is_mounted: true,
            volume: None,
            disk_space: Some(space),
        });
        seen_paths.insert(root_path);
        seen_names.insert("main system".to_string());
    }

    // 2. Dispositivos detectados por GIO VolumeMonitor (Móviles MTP, USBs, discos externos)
    let vm = gio::VolumeMonitor::get();
    let volumes = vm.volumes();

    // A. Procesar primero los montajes activos
    for mount in vm.mounts() {
        let root = mount.root();
        if let Some(path) = root.path() {
            if path == Path::new("/") || path == Path::new("/boot") || seen_paths.contains(&path) {
                continue;
            }

            let mut name = mount.name().to_string();
            let path_str = path.to_string_lossy().to_string();
            let path_lower = path_str.to_lowercase();

            // Buscar si hay un volumen asociado con un nombre más representativo
            for vol in &volumes {
                let v_name = vol.name().to_string();
                let v_lower = v_name.to_lowercase();
                let v_slug = v_lower.replace(' ', "_");
                if path_lower.contains(&v_slug) || path_lower.contains(&v_lower) || name.to_lowercase().contains(&v_lower) {
                    name = v_name;
                    break;
                }
            }

            if name == "mtp" || name.is_empty() {
                if path_lower.contains("samsung") {
                    name = "SAMSUNG Android".to_string();
                } else {
                    name = "Mobile Device (MTP)".to_string();
                }
            }

            let name_lower = name.to_lowercase();
            if seen_names.contains(&name_lower) {
                continue;
            }

            let is_phone = name_lower.contains("android")
                || name_lower.contains("phone")
                || name_lower.contains("samsung")
                || path_lower.contains("mtp")
                || path_lower.contains("gphoto2")
                || path_lower.contains("afc");

            let icon_name = if is_phone {
                "phone-symbolic"
            } else {
                "drive-removable-media-symbolic"
            };

            let disk_space = get_disk_space(&path);

            seen_paths.insert(path.clone());
            seen_names.insert(name_lower);

            devices.push(DeviceInfo {
                name,
                path: Some(path),
                icon_name,
                is_mounted: true,
                volume: mount.volume(),
                disk_space,
            });
        }
    }

    // B. Procesar volúmenes NO montados aún (evitando duplicar con montajes ya listados)
    for vol in volumes {
        if vol.get_mount().is_some() {
            continue;
        }

        let name = vol.name().to_string();
        let name_lower = name.to_lowercase();

        // Si ya tenemos un dispositivo montado con nombre similar (ej. Samsung), omitir
        if seen_names.iter().any(|s| s.contains(&name_lower) || name_lower.contains(s)) {
            continue;
        }

        let is_phone = name_lower.contains("android")
            || name_lower.contains("phone")
            || name_lower.contains("samsung");

        let icon_name = if is_phone {
            "phone-symbolic"
        } else {
            "drive-removable-media-symbolic"
        };

        seen_names.insert(name_lower);

        devices.push(DeviceInfo {
            name,
            path: None,
            icon_name,
            is_mounted: false,
            volume: Some(vol),
            disk_space: None,
        });
    }

    // C. Escanear /run/media/$USER en caso de unidades USB sin daemon de volumen activo
    if let Some(user) = std::env::var_os("USER") {
        let media_dir = PathBuf::from("/run/media").join(user);
        if media_dir.is_dir() {
            if let Ok(rd) = std::fs::read_dir(&media_dir) {
                for entry in rd.flatten() {
                    let p = entry.path();
                    if p.is_dir() && !seen_paths.contains(&p) {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let name_lower = name.to_lowercase();
                        if !seen_names.contains(&name_lower) {
                            let space = get_disk_space(&p);
                            seen_paths.insert(p.clone());
                            seen_names.insert(name_lower);
                            devices.push(DeviceInfo {
                                name,
                                path: Some(p),
                                icon_name: "drive-removable-media-symbolic",
                                is_mounted: true,
                                volume: None,
                                disk_space: space,
                            });
                        }
                    }
                }
            }
        }
    }

    devices
}
