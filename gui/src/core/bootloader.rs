//! Bootloader detection for the GRUB-to-Limine switch button.

/// Check whether the system boots via UEFI.
fn is_uefi() -> bool {
    std::path::Path::new("/sys/firmware/efi/efivars").exists()
}

/// Determine whether the GRUB-to-Limine switch button should be enabled.
///
/// Returns `Ok(())` when the switch is applicable (UEFI system, GRUB
/// installed, Limine not already installed). Returns `Err` with a
/// user-facing reason otherwise, suitable for a disabled-button tooltip.
pub fn limine_switch_availability() -> Result<(), &'static str> {
    if !is_uefi() {
        return Err("UEFI required - BIOS/legacy systems not supported.");
    }
    if !super::is_package_installed("grub") {
        return Err("GRUB not detected.");
    }
    if super::is_package_installed("limine") {
        return Err("Limine already installed.");
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
