use sysinfo::Disks;
use crate::helpers::bytes_to_megabytes;
use std::collections::HashSet;

#[derive(Debug)]
pub struct DriveInfo {
    pub name: String,
    pub mount_point: String,
    pub total_space_mb: u64,
    pub available_space_mb: u64,
    pub file_system: String,
    pub is_removable: bool,
}

pub fn get_drives_info() -> Vec<DriveInfo> {
    let disks = Disks::new_with_refreshed_list();
    let mut drives = Vec::new();

    for disk in disks.list() {
        // Skip removable drives
        if disk.is_removable() {
            continue;
        }

        let mount_point = disk.mount_point().to_string_lossy().to_string();

        // Skip macOS system volumes (except Data volume)
        if mount_point.starts_with("/System/Volumes/") && mount_point != "/System/Volumes/Data" {
            continue;
        }

        // Skip other common system-only mount points
        if mount_point.starts_with("/private/var/vm") {
            continue;
        }

        drives.push(DriveInfo {
            name: disk.name().to_string_lossy().to_string(),
            mount_point: mount_point.clone(),
            total_space_mb: bytes_to_megabytes(disk.total_space()),
            available_space_mb: bytes_to_megabytes(disk.available_space()),
            file_system: disk.file_system().to_string_lossy().to_string(),
            is_removable: disk.is_removable(),
        });
    }

    // Remove duplicate drives (same capacity) - prefer /System/Volumes/Data over /
    drives.sort_by(|a, b| {
        // Sort Data volume first
        if a.mount_point.contains("/Data") {
            std::cmp::Ordering::Less
        } else if b.mount_point.contains("/Data") {
            std::cmp::Ordering::Greater
        } else {
            a.mount_point.cmp(&b.mount_point)
        }
    });

    // Deduplicate by total_space and available_space
    let mut seen = std::collections::HashSet::new();
    drives.retain(|drive| {
        let key = (drive.total_space_mb, drive.available_space_mb);
        seen.insert(key)
    });

    drives
}
