//! SecureBoot state detection for the Servicing page's SecureBoot Setup button.

use std::process::Command;

/// Returns true only if `sbctl status` reports SecureBoot as enabled.
/// Returns false if `sbctl` is missing or its output can't be parsed -
/// an undeterminable state should not block the button.
pub fn is_secureboot_enabled() -> bool {
    let output = match Command::new("sbctl").arg("status").output() {
        Ok(output) => output,
        Err(_) => return false,
    };

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|line| line.contains("Secure Boot:") && line.contains("Enabled"))
}

/// Whether the SecureBoot Setup button should be enabled (not hidden -
/// visibility is gated separately on Limine not being installed yet).
pub fn secureboot_setup_availability() -> Result<(), &'static str> {
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
