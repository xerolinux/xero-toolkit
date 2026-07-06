# Design: SecureBoot Setup button (Servicing / System Tweaks)

## Goal

Add a "SecureBoot Setup" button to the Servicing/System Tweaks page, next to
"Switch GRUB to Limine", that launches `xsb-gui` (the standalone
`LimineSecureBoot` PyQt6 tool) - installing it first if it's not already
present - then minimizes xero-toolkit so xsb-gui is immediately visible.

## Scope

- Button only appears while Limine is NOT yet the active bootloader
  (`grub` still in place) AND SecureBoot is not already enabled. Once
  either condition flips, the button hides.
- A plain "Are you sure?" confirmation popup before proceeding - not the
  scary/destructive-styled warning used by the GRUB switch (xsb-gui itself
  already surfaces the real SecureBoot warnings), just a neutral
  Cancel/Continue check. New `show_confirmation()` dialog (see Files
  touched) rather than reusing `show_warning_confirmation`, since the
  latter's `.ui` file bakes in a permanent warning-triangle icon and a
  "Warning" window title that aren't appropriate here.
- One new `core::` helper: `core::secureboot::is_secureboot_enabled()`,
  shelling out to `sbctl status` (mirrors the detection logic in
  `~/xwork/LimineSecureBoot/lib/secureboot.sh`'s `detect_secureboot_state`).
  Reuses the existing `core::is_package_installed` check for the
  Limine/xsb-gui presence checks.

## Files touched

- `gui/resources/ui/tabs/servicing_system_tweaks.ui` - new
  `btn_secureboot_setup` GtkButton, packed into the same row as
  `btn_switch_limine`, `suggested-action pill` styling (not destructive).
- `gui/resources/data/button_info/servicing.toml` - new `[[button]]` entry
  for `btn_secureboot_setup`.
- `gui/src/ui/pages/servicing.rs` - new `setup_secureboot_setup()` handler,
  called from `setup_handlers()`.
- `gui/src/ui/dialogs/confirm.rs` (new) - `show_confirmation()`, a plain
  Cancel/Continue popup with no destructive styling, structurally mirroring
  `warning.rs`'s `show_warning_confirmation()` but without the warning icon
  or "Warning" window title.
- `gui/resources/ui/dialogs/confirm_dialog.ui` (new), registered in
  `resources.gresource.xml` and `config::resources::dialogs::CONFIRM`.

## Button gating

On page load: `btn.set_visible(!core::is_package_installed("limine") &&
!core::is_secureboot_enabled())`. Visibility only - no disabled/tooltip
state, since there's nothing actionable to explain once Limine is already
active or SecureBoot is already enabled (the button simply isn't relevant
anymore). `is_secureboot_enabled()` returns `false` (does not block
visibility) if `sbctl` isn't installed or its output can't be parsed - an
undeterminable SecureBoot state should not hide the button.

## Click flow

0. Show `show_confirmation(window, "Are you sure?", "...", on_confirm)`.
   Nothing below happens until the user clicks Continue.
1. If `core::is_package_installed("xsb-gui")` is already true: skip
   straight to step 3.
2. Otherwise, run a 1-step `CommandSequence` (`.aur()` install of
   `xsb-gui`) via `task_runner::run(...)`, showing the normal progress
   dialog. Since `task_runner::run` has no completion callback, poll
   `task_runner::is_running()` on a 1-second `glib::timeout_add_seconds_local`
   (same pattern as `kernel_manager_tab.rs`'s post-install refresh) until it
   returns false, then re-check `core::is_package_installed("xsb-gui")`.
   - If still false (install failed or was cancelled), stop here. The
     task_runner dialog has already shown the failure; no further action.
3. Spawn `xsb-gui` detached (`std::process::Command::new("xsb-gui")` with
   stdin/stdout/stderr set to `Stdio::null()`, `.spawn()`, `error!` logged on
   failure), matching the existing `xfprintd-gui`/`xero-howdy-qt` launch
   pattern in `biometrics.rs`. Then call `window.minimize()` (GTK4
   `GtkWindowExt::minimize`).

## Error handling

No new error-handling layer. Install failure/cancellation is already
surfaced by `task_runner`'s own dialog (same as every other button); this
handler only adds the guard in step 2 so it doesn't spawn/minimize after a
failed install. A `spawn()` failure for `xsb-gui` itself is logged via
`error!`, same as the existing biometrics launch pattern - no user-facing
dialog for that case, consistent with existing precedent.

## Testing

Manual verification on a real XeroLinux install:
- Button visible on a GRUB system, hidden on a Limine system.
- Click shows the plain "Are you sure?" confirmation popup (no warning
  icon, no "Warning" window title).
- Cancel the confirmation: nothing happens, no install, no minimize.
- Confirm with xsb-gui not installed: progress dialog installs it, window
  minimizes, xsb-gui window appears.
- Confirm with xsb-gui already installed: no progress dialog, window
  minimizes immediately, xsb-gui window appears.
- Cancel/fail the install (after confirming): window does not minimize,
  no crash.
