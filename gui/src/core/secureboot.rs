//! SecureBoot state detection for the Servicing page's SecureBoot Setup button.

use std::process::Command;

/// Returns true only if `sbctl status` reports SecureBoot as enabled.
/// Returns false if `sbctl` is missing or its output can't be parsed -
/// an undeterminable state should not block the button.
fn is_secureboot_enabled() -> bool {
    let output = match Command::new("sbctl").arg("status").output() {
        Ok(output) => output,
        Err(_) => return false,
    };

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|line| line.contains("Secure Boot:") && line.contains("Enabled"))
}

/// Determine whether the SecureBoot Setup button should be enabled.
///
/// Returns `Ok(())` when setup is still applicable (Limine not yet active,
/// SecureBoot not already enabled). Returns `Err` with a user-facing reason
/// otherwise, suitable for a disabled-button tooltip - the button is always
/// visible, never hidden, matching `limine_switch_availability`'s
/// disable-with-tooltip convention.
pub fn secureboot_setup_availability() -> Result<(), &'static str> {
    if super::is_package_installed("limine") {
        return Err("Limine is already active - nothing to set up.");
    }
    if is_secureboot_enabled() {
        return Err("SecureBoot is already enabled.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_secureboot_enabled_does_not_panic() {
        let _ = is_secureboot_enabled();
    }
}
