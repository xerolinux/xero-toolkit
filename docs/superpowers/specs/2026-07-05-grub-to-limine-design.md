# Design: Switch GRUB to Limine (Servicing / System Tweaks)

## Goal

Add a button to the Servicing/System Tweaks page that performs a one-way,
irreversible switch of the bootloader from GRUB to Limine: back up all
GRUB-related files, remove GRUB/os-prober/update-grub and their configs
(including themes), then install and activate Limine + limine-mkinitcpio-hook.

## Scope

- UEFI systems only. BIOS/legacy is out of scope; button stays disabled with
  an explanatory tooltip on non-UEFI systems.
- One-directional action (GRUB -> Limine). No Limine -> GRUB reverse path.
- No new abstractions beyond what existing servicing.rs / drivers.rs buttons
  already use (Command/CommandSequence/task_runner, show_warning_confirmation).

## Files touched

- `sources/scripts/switch_grub_to_limine.sh` (new) - backup + purge + Limine
  EFI deployment logic, mirrors the single-purpose script convention already
  used by `nvidia_grub.sh` / `nvidia_mkinitcpio.sh`.
- `gui/src/ui/pages/servicing.rs` - new `setup_switch_to_limine()` handler,
  called from `setup_handlers()`.
- `gui/resources/ui/tabs/servicing_system_tweaks.ui` - new
  `btn_switch_limine` GtkButton, added to the existing button grid.
- `gui/src/core/bootloader.rs` (new) - `is_grub_active() -> bool`,
  `is_uefi() -> bool` detection helpers, following the style of
  `core/package.rs`.

## Button gating

On page load, the button is enabled only if all of:
- `core::is_package_installed("grub")` is true
- `core::is_package_installed("limine")` is false
- `/sys/firmware/efi/efivars` exists (UEFI)

Otherwise the button is disabled with a tooltip explaining which condition
failed ("GRUB not detected", "Limine already installed", or "UEFI required -
BIOS/legacy systems not supported").

## Click flow

1. `show_warning_confirmation` dialog states plainly: this is irreversible,
   GRUB will be backed up to `/root/grub-backup-<timestamp>.tar.gz`, then
   removed along with os-prober/update-grub and all configs/themes, Limine
   will be installed and activated, and a reboot is required afterward.
2. On confirm, a `CommandSequence` runs:
   - **Step 1** (`.aur()`): install `["limine", "limine-mkinitcpio-hook"]`.
     Using the AUR helper (paru/yay) for both mirrors the existing pattern
     for mixed official/AUR package sets (e.g. ASUS ROG button in
     drivers.rs), regardless of which repo each package actually lives in.
   - **Step 2** (`.privileged()`): run `switch_grub_to_limine.sh`. This
     backs up, removes grub/os-prober/update-grub packages and leftover
     files, then deploys the Limine EFI binary and registers it with
     `efibootmgr`.
   - **Step 3** (`.privileged()`): `mkinitcpio -P` to regenerate initramfs;
     `limine-mkinitcpio-hook` writes the `limine.conf` boot entries as part
     of this run.

Step ordering matters: Limine must already be installed (step 1) before step
2 can deploy its EFI binary, and the mkinitcpio hook must be registered
(step 1) before step 3 regenerates `limine.conf`.

## `switch_grub_to_limine.sh` logic

- **ESP detection**: try `findmnt -n --target /boot/efi`, then `/efi`, then
  `/boot`, using the first that resolves to a mounted vfat/ESP filesystem.
- **Backup**: `tar czf /root/grub-backup-<timestamp>.tar.gz` covering
  `/etc/default/grub`, `/etc/grub.d/`, `/boot/grub/` (includes themes), and
  `$ESP/EFI/GRUB` if present. Each path is guarded with an existence check
  so a missing path doesn't fail the tar.
- **Removal**: `pacman -Rns --noconfirm grub os-prober`, plus `update-grub`
  if that package is installed. Then explicit `rm -rf` of
  `/etc/default/grub /etc/grub.d /boot/grub $ESP/EFI/GRUB` to clear
  `.pacsave` leftovers and any unpackaged theme files pacman doesn't own.
- **Limine deployment**: copy the Limine EFI binary to
  `$ESP/EFI/limine/BOOTX64.EFI`, then register a boot entry via
  `efibootmgr --create --disk <disk> --part <part> --loader '\EFI\limine\BOOTX64.EFI' --label 'Limine Bootloader' --unicode`.
- `set -e` throughout; all destructive operations happen only after the
  backup step has succeeded, so a failed backup aborts before anything is
  deleted.

## Error handling

No new error-handling layer. Existing `task_runner` behavior already halts
the sequence and reports the failing step/exit code; this button relies on
that as-is, same as every other multi-step button in servicing.rs/drivers.rs.

## Testing

Manual verification on a real (or VM) UEFI Arch/XeroLinux install running
GRUB:
- Button disabled state on a Limine system and on a BIOS system.
- Full run on a GRUB+UEFI system: confirm backup tar exists and contains
  the expected paths, confirm grub/os-prober packages gone, confirm
  `/boot/grub` and ESP GRUB dir gone, confirm system boots into Limine after
  reboot, confirm `limine.conf` lists the installed kernels.
