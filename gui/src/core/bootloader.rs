//! Bootloader detection for the GRUB-to-Limine switch button.

use std::process::Command;

/// Minimum ESP size to safely fit Limine, multiple signed kernels, and the
/// boot splash - matches the same floor LimineSecureBoot's own wizard uses
/// (`xsb_gui/esp_size_check.py`'s `ESP_MIN_SIZE_BYTES`).
const ESP_MIN_SIZE_BYTES: u64 = 1024 * 1024 * 1024;

/// Check whether the system boots via UEFI.
fn is_uefi() -> bool {
    std::path::Path::new("/sys/firmware/efi/efivars").exists()
}

/// Find the EFI System Partition mountpoint. Matched by mountpoint + vfat
/// fstype (same check `switch_grub_to_limine.sh` already uses successfully)
/// rather than checking for an `EFI` subdirectory - this function runs
/// unprivileged from the GUI process, and ESP mounts are commonly
/// root-restricted (0700), so a directory-traversal check would silently
/// fail here even on a perfectly valid ESP.
fn find_esp_mountpoint() -> Option<&'static str> {
    let output = Command::new("findmnt")
        .args(["-rno", "TARGET"])
        .output()
        .ok()?;
    let mounted = String::from_utf8_lossy(&output.stdout);
    ["/boot/efi", "/efi", "/boot"]
        .into_iter()
        .find(|candidate| mounted.lines().any(|line| line == *candidate) && is_vfat(candidate))
}

fn is_vfat(mountpoint: &str) -> bool {
    let Ok(output) = Command::new("findmnt")
        .args(["-no", "FSTYPE", mountpoint])
        .output()
    else {
        return false;
    };
    String::from_utf8_lossy(&output.stdout).trim() == "vfat"
}

fn esp_source_device(esp_mountpoint: &str) -> Option<String> {
    let output = Command::new("findmnt")
        .args(["-no", "SOURCE", esp_mountpoint])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let source = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!source.is_empty()).then_some(source)
}

/// Whether the ESP's parent disk uses a GPT partition table, mirroring
/// `preflight.sh`'s `detect_partition_table` - UEFI firmware requires GPT,
/// so a BIOS/MBR-style disk can't host a working Limine install.
fn is_gpt() -> bool {
    let Some(esp) = find_esp_mountpoint() else {
        return false;
    };
    let Some(esp_source) = esp_source_device(esp) else {
        return false;
    };
    let Ok(output) = Command::new("lsblk")
        .args(["-no", "PKNAME", &esp_source])
        .output()
    else {
        return false;
    };
    let parent_disk = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if parent_disk.is_empty() {
        return false;
    }
    let Ok(output) = Command::new("lsblk")
        .args(["-dno", "PTTYPE", &format!("/dev/{parent_disk}")])
        .output()
    else {
        return false;
    };
    String::from_utf8_lossy(&output.stdout).trim() == "gpt"
}

/// Total ESP size in bytes, or `None` if undeterminable - matching
/// `preflight.sh`'s `detect_esp_size_bytes`. An undeterminable size must
/// not block the button (same non-blocking convention as `secureboot.rs`).
fn esp_size_bytes() -> Option<u64> {
    let esp = find_esp_mountpoint()?;
    let esp_source = esp_source_device(esp)?;
    let output = Command::new("lsblk")
        .args(["-bdno", "SIZE", &esp_source])
        .output()
        .ok()?;
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

/// Determine whether the GRUB-to-Limine switch button should be enabled.
///
/// Returns `Ok(())` when the switch is applicable (UEFI system, GPT disk,
/// GRUB installed, Limine not already installed, ESP large enough). Returns
/// `Err` with a user-facing reason otherwise, suitable for a disabled-button
/// tooltip.
pub fn limine_switch_availability() -> Result<(), &'static str> {
    if !is_uefi() {
        return Err("UEFI required - BIOS/legacy systems not supported.");
    }
    if !is_gpt() {
        return Err("Disk is not GPT - UEFI firmware requires a GPT partition table.");
    }
    if !super::is_package_installed("grub") {
        return Err("GRUB not detected.");
    }
    if super::is_package_installed("limine") {
        return Err("Limine already installed.");
    }
    if let Some(size) = esp_size_bytes() {
        if size < ESP_MIN_SIZE_BYTES {
            return Err(
                "EFI system partition is smaller than 1024MB - not enough room for Limine, kernels, and the boot splash.",
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limine_switch_availability_does_not_panic() {
        let _ = limine_switch_availability();
    }

    #[test]
    fn test_is_uefi_matches_efivars_path() {
        let expected = std::path::Path::new("/sys/firmware/efi/efivars").exists();
        assert_eq!(is_uefi(), expected);
    }
}
