use sysinfo::Disks;
use crate::helpers::bytes_to_megabytes;

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
        drives.push(DriveInfo {
            name: disk.name().to_string_lossy().to_string(),
            mount_point: disk.mount_point().to_string_lossy().to_string(),
            total_space_mb: bytes_to_megabytes(disk.total_space()),
            available_space_mb: bytes_to_megabytes(disk.available_space()),
            file_system: String::from_utf8_lossy(disk.file_system()).to_string(),
            is_removable: disk.is_removable(),
        });
    }

    drives
}
