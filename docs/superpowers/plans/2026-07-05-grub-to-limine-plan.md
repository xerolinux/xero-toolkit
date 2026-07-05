# GRUB to Limine Switch Button Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Servicing/System Tweaks page button that irreversibly switches
a UEFI system's bootloader from GRUB to Limine, backing up GRUB first and
removing every GRUB-related package and file afterward.

**Architecture:** A new `core::bootloader` module gates the button
(UEFI + GRUB-present + Limine-absent). A new standalone bash script does the
actual backup/removal/Limine-deployment work, run as a privileged step from
a `CommandSequence`, following the exact pattern already used by
`nvidia_grub.sh` and the Servicing/Drivers page buttons.

**Tech Stack:** Rust (GTK4 + libadwaita), bash, pacman/AUR helper
(paru/yay), existing `task_runner`/`Command`/`CommandSequence` execution
layer.

## Global Constraints

- UEFI systems only; BIOS/legacy is out of scope. Button stays disabled with
  an explanatory tooltip on non-UEFI systems.
- One-directional action (GRUB -> Limine). No reverse path.
- No new abstractions beyond what servicing.rs/drivers.rs buttons already
  use: `Command`/`CommandSequence`/`task_runner::run`,
  `show_warning_confirmation`.
- Button enabled only if: `grub` package installed AND `limine` package NOT
  installed AND `/sys/firmware/efi/efivars` exists.
- Backup target: `/root/grub-backup-<timestamp>.tar.gz`, covering
  `/etc/default/grub`, `/etc/grub.d/`, `/boot/grub/` (incl. themes), and the
  ESP's `EFI/GRUB` directory if present.
- Removed packages: `grub`, `os-prober`, and `update-grub` (only if
  installed). Leftover `.pacsave` configs and unpackaged theme files are
  explicitly deleted after package removal.
- Limine packages installed via the AUR helper (`.aur()`), matching the
  existing convention of using the helper for mixed official/AUR package
  sets regardless of which repo each package lives in: `limine`,
  `limine-mkinitcpio-hook`.
- All destructive operations happen only after the backup step succeeds
  (`set -e` in the script).

---

### Task 1: Bootloader detection helper

**Files:**
- Create: `gui/src/core/bootloader.rs`
- Modify: `gui/src/core/mod.rs`
- Test: inline `#[cfg(test)]` module in `gui/src/core/bootloader.rs`
  (matches existing style in `gui/src/core/package.rs` and
  `gui/src/core/aur.rs` — no separate test file in this crate)

**Interfaces:**
- Consumes: `crate::core::is_package_installed(package: &str) -> bool`
  (already defined in `gui/src/core/package.rs:11`)
- Produces: `crate::core::limine_switch_availability() -> Result<(), &'static str>`
  — `Ok(())` when the button should be enabled, `Err(reason)` with a
  user-facing tooltip string otherwise. Task 3 calls this directly.

- [ ] **Step 1: Write the failing tests**

Create `gui/src/core/bootloader.rs` with just the test module first:

```rust
//! Bootloader detection for the GRUB-to-Limine switch button.

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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p xero-toolkit core::bootloader -- --nocapture`
Expected: FAIL to compile with `cannot find function 'limine_switch_availability'` / `cannot find function 'is_uefi'`

- [ ] **Step 3: Write the implementation**

Add above the test module in `gui/src/core/bootloader.rs`:

```rust
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
```

- [ ] **Step 4: Register the module**

Modify `gui/src/core/mod.rs`:

```rust
pub mod aur;
pub mod autostart;
pub mod bootloader;
pub mod daemon;
pub mod download;
pub mod package;
pub mod system_check;

// Re-export commonly used items
pub use aur::get as aur_helper;
pub use bootloader::limine_switch_availability;
pub use package::{is_flatpak_installed, is_package_installed};
pub use system_check::{check_dependencies, get_distribution_name, show_dependency_error_dialog};
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p xero-toolkit core::bootloader -- --nocapture`
Expected: PASS (2 tests)

- [ ] **Step 6: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: no changes needed from fmt, no clippy warnings

- [ ] **Step 7: Commit**

```bash
git add gui/src/core/bootloader.rs gui/src/core/mod.rs
git commit -m "feat: add bootloader detection for GRUB-to-Limine gating"
```

---

### Task 2: switch_grub_to_limine.sh script

**Files:**
- Create: `sources/scripts/switch_grub_to_limine.sh`

**Interfaces:**
- Consumes: nothing from prior tasks (standalone script, run as root by the
  privileged command layer).
- Produces: an executable at
  `crate::config::paths::scripts().join("switch_grub_to_limine.sh")`
  (`/opt/xero-toolkit/sources/scripts/switch_grub_to_limine.sh` at runtime)
  that Task 3's `Command::builder().privileged()` step invokes via
  `bash <path>`.

This script assumes the `limine` package is already installed (Task 3's
command sequence installs it in the step before running this script), so
`/usr/share/limine/BOOTX64.EFI` is available.

- [ ] **Step 1: Write the script**

Create `sources/scripts/switch_grub_to_limine.sh`:

```bash
#!/bin/bash
set -e

TIMESTAMP=$(date +%Y.%m.%d-%H.%M.%S)
BACKUP_FILE="/root/grub-backup-${TIMESTAMP}.tar.gz"

# Locate the EFI System Partition mountpoint.
ESP=""
for candidate in /boot/efi /efi /boot; do
  if findmnt -n --target "$candidate" >/dev/null 2>&1; then
    ESP="$candidate"
    break
  fi
done

if [[ -z "$ESP" ]]; then
  echo "Could not locate EFI System Partition mountpoint" >&2
  exit 1
fi

# Back up everything GRUB-related before touching anything.
BACKUP_PATHS=()
for path in /etc/default/grub /etc/grub.d /boot/grub "${ESP}/EFI/GRUB" "${ESP}/EFI/grub"; do
  [[ -e "$path" ]] && BACKUP_PATHS+=("$path")
done

if [[ ${#BACKUP_PATHS[@]} -gt 0 ]]; then
  tar czf "$BACKUP_FILE" "${BACKUP_PATHS[@]}"
  echo "GRUB backup saved to $BACKUP_FILE"
else
  echo "No GRUB files found to back up"
fi

# Remove grub, and os-prober/update-grub only if actually installed.
REMOVE_PKGS=(grub)
for pkg in os-prober update-grub; do
  if pacman -Q "$pkg" >/dev/null 2>&1; then
    REMOVE_PKGS+=("$pkg")
  fi
done
pacman -Rns --noconfirm "${REMOVE_PKGS[@]}"

# Clear leftover config/theme files pacman leaves as .pacsave or never owned.
rm -rf /etc/default/grub /etc/default/grub.pacsave /etc/grub.d /boot/grub \
  "${ESP}/EFI/GRUB" "${ESP}/EFI/grub"

# Deploy Limine to the ESP.
mkdir -p "${ESP}/EFI/limine"
cp /usr/share/limine/BOOTX64.EFI "${ESP}/EFI/limine/BOOTX64.EFI"

# Register a boot entry via efibootmgr.
ESP_SOURCE=$(findmnt -n -o SOURCE "$ESP")
ESP_DISK=$(lsblk -no PKNAME "$ESP_SOURCE" | head -n1)
ESP_PART=$(lsblk -no PARTNUM "$ESP_SOURCE")

if [[ -n "$ESP_DISK" && -n "$ESP_PART" ]]; then
  efibootmgr --create --disk "/dev/${ESP_DISK}" --part "$ESP_PART" \
    --loader '\EFI\limine\BOOTX64.EFI' --label 'Limine Bootloader' --unicode
else
  echo "Could not determine ESP disk/partition for efibootmgr registration" >&2
  exit 1
fi

echo "Limine installed. Run mkinitcpio -P to generate limine.conf entries."
```

- [ ] **Step 2: Make it executable**

Run: `chmod +x sources/scripts/switch_grub_to_limine.sh`

- [ ] **Step 3: Verify shell syntax**

Run: `bash -n sources/scripts/switch_grub_to_limine.sh`
Expected: no output, exit code 0

- [ ] **Step 4: Commit**

```bash
git add sources/scripts/switch_grub_to_limine.sh
git commit -m "feat: add GRUB backup/removal and Limine deploy script"
```

---

### Task 3: Servicing page button (UI + handler wiring)

**Files:**
- Modify: `gui/resources/ui/tabs/servicing_system_tweaks.ui`
- Modify: `gui/src/ui/pages/servicing.rs`
- Modify: `gui/resources/data/button_info/servicing.toml`

**Interfaces:**
- Consumes:
  - `crate::core::limine_switch_availability() -> Result<(), &'static str>`
    (Task 1)
  - `crate::config::paths::scripts() -> PathBuf` (existing,
    `gui/src/config/constants.rs:51`)
  - `sources/scripts/switch_grub_to_limine.sh` (Task 2)
  - `crate::ui::dialogs::warning::show_warning_confirmation` (existing,
    `gui/src/ui/dialogs/warning.rs:13`)
  - `crate::ui::task_runner::{self, Command, CommandSequence}` (existing)
- Produces: nothing consumed by later tasks (this is the terminal task).

- [ ] **Step 1: Add the button to the UI resource**

Modify `gui/resources/ui/tabs/servicing_system_tweaks.ui`, adding a new row
after the "Row 3" block (after the closing `</object>`/`</child>` for the
`btn_parallel_downloads` row, i.e. right before the two closing
`</child>`/`</object>` tags that close the outer buttons box and page box):

```xml
        <!-- Row 4: Switch GRUB to Limine -->
        <child>
          <object class="GtkBox">
            <property name="orientation">horizontal</property>
            <property name="spacing">16</property>
            <property name="halign">center</property>
            <child>
              <object class="GtkButton" id="btn_switch_limine">
                <property name="label">Switch GRUB to Limine</property>
                <property name="width-request">200</property>
                <property name="height-request">50</property>
                <property name="css-classes">destructive-action pill</property>
              </object>
            </child>
          </object>
        </child>
```

Use `destructive-action` (not `suggested-action`) since this is the
irreversible/destructive button, distinguishing it visually from the other
Servicing buttons.

- [ ] **Step 2: Verify the resource compiles**

Run: `cargo build -p xero-toolkit`
Expected: build succeeds (glib-compile-resources bundled via `build.rs`
fails loudly on malformed XML, so a clean build confirms the markup is
well-formed)

- [ ] **Step 3: Add the button_info entry**

Append to `gui/resources/data/button_info/servicing.toml`:

```toml
[[button]]
id = "btn_switch_limine"
title = "Switch GRUB to Limine"
summary = "Irreversibly backs up and removes GRUB, then installs and activates Limine."
details = [
  "Backs up /etc/default/grub, /etc/grub.d, /boot/grub, and the ESP GRUB directory to /root/grub-backup-<timestamp>.tar.gz.",
  "Removes grub, os-prober, and update-grub (if present), then deletes leftover configs and themes.",
  "Installs limine and limine-mkinitcpio-hook, deploys the Limine EFI binary, and registers it with efibootmgr.",
  "Runs mkinitcpio -P afterward so limine-mkinitcpio-hook generates limine.conf boot entries.",
]
caution = "Irreversible. UEFI systems only. A reboot is required after this completes."
```

- [ ] **Step 4: Write the click handler**

Modify `gui/src/ui/pages/servicing.rs`. Add to the top-of-file doc comment
list and to `setup_handlers`:

```rust
//! - Update mirrorlist
//! - Parallel downloads adjustment
//! - Switch GRUB to Limine
```

```rust
pub fn setup_handlers(page_builder: &Builder, _main_builder: &Builder, window: &ApplicationWindow) {
    crate::ui::dialogs::button_info::attach_to_builder(page_builder, window.upcast_ref());
    setup_clr_pacman(page_builder, window);
    setup_unlock_pacman(page_builder, window);
    setup_plasma_x11(page_builder, window);
    setup_pacman_db_fix(page_builder, window);
    setup_fix_gpgme(page_builder, window);
    setup_fix_arch_keyring(page_builder, window);
    setup_update_mirrorlist(page_builder, window);
    setup_parallel_downloads(page_builder, window);
    setup_switch_to_limine(page_builder, window);
}
```

Add the new function (place after `setup_parallel_downloads`):

```rust
fn setup_switch_to_limine(page_builder: &Builder, window: &ApplicationWindow) {
    use crate::ui::dialogs::warning::show_warning_confirmation;

    let btn_switch_limine = extract_widget::<gtk4::Button>(page_builder, "btn_switch_limine");

    match core::limine_switch_availability() {
        Ok(()) => {
            btn_switch_limine.set_sensitive(true);
        }
        Err(reason) => {
            btn_switch_limine.set_sensitive(false);
            btn_switch_limine.set_tooltip_text(Some(reason));
        }
    }

    let window = window.clone();
    btn_switch_limine.connect_clicked(move |_| {
        info!("Servicing: Switch GRUB to Limine button clicked");

        let window_clone = window.clone();
        show_warning_confirmation(
            window.upcast_ref(),
            "Switch GRUB to Limine",
            "This is <b>irreversible</b>.\n\n\
            GRUB will be backed up to <span foreground=\"cyan\">/root/grub-backup-&lt;timestamp&gt;.tar.gz</span>, \
            then GRUB, os-prober, and update-grub (if present) will be removed along with all configs and themes.\n\n\
            Limine and limine-mkinitcpio-hook will then be installed and activated.\n\n\
            <span foreground=\"red\" weight=\"bold\">A reboot is required</span> after this completes.",
            move || {
                let script = crate::config::paths::scripts()
                    .join("switch_grub_to_limine.sh")
                    .to_string_lossy()
                    .into_owned();

                let commands = CommandSequence::new()
                    .then(
                        Command::builder()
                            .aur()
                            .args(&["-S", "--noconfirm", "--needed", "limine", "limine-mkinitcpio-hook"])
                            .description("Installing Limine...")
                            .build(),
                    )
                    .then(
                        Command::builder()
                            .privileged()
                            .program("bash")
                            .args(&[&script])
                            .description("Backing up and removing GRUB, deploying Limine...")
                            .build(),
                    )
                    .then(
                        Command::builder()
                            .privileged()
                            .program("mkinitcpio")
                            .args(&["-P"])
                            .description("Rebuilding initramfs and generating Limine boot entries...")
                            .build(),
                    )
                    .build();

                task_runner::run(
                    window_clone.upcast_ref(),
                    commands,
                    "Switch GRUB to Limine",
                );
            },
        );
    });
}
```

- [ ] **Step 5: Build**

Run: `cargo build -p xero-toolkit`
Expected: build succeeds with no errors

- [ ] **Step 6: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: no changes needed from fmt, no clippy warnings

- [ ] **Step 7: Manual verification**

This is a GTK4 desktop app requiring a display; there is no existing
automated UI test harness for this codebase (button clicks/dialogs are
verified manually for every button in servicing.rs/drivers.rs). Run the app
locally and confirm:
- On a non-UEFI or non-Arch dev machine: button is disabled, tooltip shows
  the correct unavailability reason.
- Right-click the button: info dialog shows the servicing.toml entry
  content from Step 3.
- (On a real UEFI+GRUB Arch/XeroLinux system only) Clicking the button
  shows the warning dialog; confirming runs the 3-step sequence in the
  task-runner progress window; verify the backup tar exists, grub/os-prober
  are gone, and `/boot/grub` + ESP `EFI/GRUB` are gone before rebooting to
  confirm Limine boots.

- [ ] **Step 8: Commit**

```bash
git add gui/resources/ui/tabs/servicing_system_tweaks.ui \
        gui/src/ui/pages/servicing.rs \
        gui/resources/data/button_info/servicing.toml
git commit -m "feat: add Switch GRUB to Limine button to Servicing page"
```
