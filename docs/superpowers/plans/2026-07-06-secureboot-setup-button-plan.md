# SecureBoot Setup Button Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a "SecureBoot Setup" button next to "Switch GRUB to Limine" on
the Servicing/System Tweaks page that installs `xsb-gui` if needed, then
minimizes xero-toolkit and launches it.

**Architecture:** A single new handler in `servicing.rs`, following the
exact patterns already in this codebase: `core::is_package_installed` and
a new `core::secureboot::is_secureboot_enabled()` for gating/detection,
`CommandSequence`/`task_runner::run` for the AUR install, the
`kernel_manager_tab.rs` polling pattern to run code after
`task_runner::run` finishes (it has no completion callback), and the
`biometrics.rs` detached-spawn pattern to launch the external GUI.

**Tech Stack:** Rust (GTK4 + libadwaita), AUR helper (paru/yay), existing
`task_runner`/`Command`/`CommandSequence` execution layer, `gtk4::glib`
timeout polling.

## Global Constraints

- Button visible only when `core::is_package_installed("limine")` is
  false AND `core::is_secureboot_enabled()` is false; hidden once Limine
  is the active bootloader or SecureBoot is already enabled (not
  disabled/tooltip - this button is irrelevant, not merely blocked, once
  either condition is true).
- A plain "Are you sure?" confirmation popup before proceeding (not the
  scary destructive-styled warning used by the GRUB switch - xsb-gui
  itself already surfaces the real SecureBoot warnings). New
  `gui/src/ui/dialogs/confirm.rs`'s `show_confirmation()`, not a reuse of
  `show_warning_confirmation` (whose `.ui` file bakes in a permanent
  warning-triangle icon and "Warning" window title).
- One new `core::` module, `core::secureboot`, exposing
  `is_secureboot_enabled() -> bool` (shells out to `sbctl status`,
  mirrors `~/xwork/LimineSecureBoot/lib/secureboot.sh`'s detection
  logic; returns `false` - does not block visibility - if `sbctl` is
  missing or unparseable). Reuse `core::is_package_installed` for the
  Limine-present and xsb-gui-present checks.
- `xsb-gui` installed via `.aur()`, matching the existing convention of
  routing all package installs through the AUR helper regardless of which
  repo the package actually lives in.
- After a successful install (or if already installed), spawn `xsb-gui`
  detached and call `window.minimize()`. If install fails or is
  cancelled, do neither.

---

### Task 1: SecureBoot Setup button (UI, info dialog, handler)

**Files:**
- Modify: `gui/resources/ui/tabs/servicing_system_tweaks.ui:145-160` (Row 4)
- Modify: `gui/resources/data/button_info/servicing.toml` (append new
  `[[button]]` entry)
- Modify: `gui/src/ui/pages/servicing.rs` (new `setup_secureboot_setup()`
  handler, called from `setup_handlers()`)
- Create: `gui/src/core/secureboot.rs` (`is_secureboot_enabled() -> bool`)
- Modify: `gui/src/core/mod.rs` (add `pub mod secureboot;` and
  `pub use secureboot::is_secureboot_enabled;`)
- Create: `gui/src/ui/dialogs/confirm.rs` (`show_confirmation()`)
- Create: `gui/resources/ui/dialogs/confirm_dialog.ui`
- Modify: `gui/src/ui/dialogs/mod.rs` (add `pub mod confirm;`)
- Modify: `gui/src/config/constants.rs` (add
  `resources::dialogs::CONFIRM`)
- Modify: `gui/resources/resources.gresource.xml` (register
  `ui/dialogs/confirm_dialog.ui`)

**Interfaces:**
- Consumes: `crate::core::is_package_installed(package: &str) -> bool`
  (`gui/src/core/package.rs:11`), `task_runner::run(parent: &Window,
  commands: CommandSequence, title: &str)`, `task_runner::is_running() ->
  bool` (`gui/src/ui/task_runner/mod.rs:136`).
- Produces: nothing consumed by later tasks - this is the only task.

- [ ] **Step 1: Add the button to the UI file**

Edit `gui/resources/ui/tabs/servicing_system_tweaks.ui`. Replace the Row 4
comment and box (currently lines 145-160) with:

```xml
        <!-- Row 4: Switch GRUB to Limine, SecureBoot Setup -->
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
            <child>
              <object class="GtkButton" id="btn_secureboot_setup">
                <property name="label">SecureBoot Setup</property>
                <property name="width-request">200</property>
                <property name="height-request">50</property>
                <property name="css-classes">suggested-action pill</property>
                <property name="visible">false</property>
              </object>
            </child>
          </object>
        </child>
```

`visible` defaults to `false` in the `.ui` file itself so there's no
flash-of-visible-button before the handler runs its gating check on
startup.

- [ ] **Step 2: Add the button_info entry**

Append to `gui/resources/data/button_info/servicing.toml`:

```toml
[[button]]
id = "btn_secureboot_setup"
title = "SecureBoot Setup"
summary = "Installs xsb-gui if needed, then minimizes the toolkit and launches it."
details = [
  "Hidden once Limine is active or SecureBoot is already enabled.",
  "Shows a simple confirmation popup before proceeding.",
  "Installs xsb-gui via the AUR helper if it isn't already present.",
  "Minimizes xero-toolkit and launches xsb-gui for SecureBoot setup.",
]
```

Add a new plain confirmation dialog, `gui/resources/ui/dialogs/confirm_dialog.ui`
(mirrors `warning_dialog.ui` structurally, minus the warning-triangle icon,
with a neutral `"Xero Toolkit"` window title instead of `"... - Warning"`):

```xml
<?xml version="1.0" encoding="UTF-8"?>
<interface>
  <requires lib="gtk" version="4.0"/>
  <requires lib="adw" version="1.0"/>
  <object class="AdwWindow" id="confirm_dialog">
    <property name="title">Xero Toolkit</property>
    <property name="default-width">400</property>
    <property name="default-height">-1</property>
    <property name="modal">true</property>
    <property name="resizable">false</property>
    <property name="content">
      <object class="AdwToolbarView">
        <child type="top">
          <object class="AdwHeaderBar">
            <property name="show-title">true</property>
            <property name="show-end-title-buttons">true</property>
          </object>
        </child>
        <property name="content">
          <object class="GtkBox" id="confirm_root_box">
            <property name="orientation">vertical</property>
            <property name="spacing">20</property>
            <property name="margin-top">28</property>
            <property name="margin-bottom">28</property>
            <property name="margin-start">30</property>
            <property name="margin-end">30</property>
            <child>
              <object class="GtkLabel" id="dialog_heading">
                <property name="label">Are you sure?</property>
                <property name="halign">center</property>
                <style>
                  <class name="title-2"/>
                </style>
              </object>
            </child>
            <child>
              <object class="GtkLabel" id="confirm_message">
                <property name="use-markup">true</property>
                <property name="wrap">true</property>
                <property name="wrap-mode">word</property>
                <property name="justify">left</property>
                <property name="xalign">0</property>
                <property name="halign">fill</property>
                <property name="hexpand">true</property>
                <property name="vexpand">true</property>
                <property name="width-chars">40</property>
                <property name="label"><![CDATA[]]></property>
              </object>
            </child>
            <child>
              <object class="GtkBox" id="confirm_button_box">
                <property name="orientation">horizontal</property>
                <property name="halign">center</property>
                <property name="spacing">12</property>
                <child>
                  <object class="GtkButton" id="cancel_button">
                    <property name="label">Cancel</property>
                    <property name="width-request">100</property>
                  </object>
                </child>
                <child>
                  <object class="GtkButton" id="continue_button">
                    <property name="label">Continue</property>
                    <property name="width-request">100</property>
                    <style>
                      <class name="suggested-action"/>
                    </style>
                  </object>
                </child>
              </object>
            </child>
          </object>
        </property>
      </object>
    </property>
  </object>
</interface>
```

Register it in `gui/resources/resources.gresource.xml` (add a line for
`ui/dialogs/confirm_dialog.ui` alongside the other dialog entries, e.g.
right after `ui/dialogs/warning_dialog.ui`), and in
`gui/src/config/constants.rs`'s `resources::dialogs` module, add:

```rust
pub const CONFIRM: &str = "/xyz/xerolinux/xero-toolkit/ui/dialogs/confirm_dialog.ui";
```

Add `gui/src/ui/dialogs/confirm.rs`:

```rust
//! Plain "Are you sure?" confirmation dialog for non-destructive actions.

use crate::ui::utils::extract_widget;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Builder, Button, Label, Window};
use log::info;
use std::cell::RefCell;
use std::rc::Rc;

/// Show a plain confirmation dialog with cancel and continue buttons.
/// Calls on_confirm callback if user clicks continue.
pub fn show_confirmation<F>(parent: &Window, heading: &str, message: &str, on_confirm: F)
where
    F: FnOnce() + 'static,
{
    info!("Showing confirmation dialog: {}", heading);

    let builder = Builder::from_resource(crate::config::resources::dialogs::CONFIRM);

    let dialog: Window = extract_widget(&builder, "confirm_dialog");
    dialog.set_transient_for(Some(parent));

    let heading_label: Label = extract_widget(&builder, "dialog_heading");
    let confirm_message: Label = extract_widget(&builder, "confirm_message");
    let cancel_button: Button = extract_widget(&builder, "cancel_button");
    let continue_button: Button = extract_widget(&builder, "continue_button");

    heading_label.set_label(heading);
    confirm_message.set_markup(message);
    confirm_message.connect_activate_link(|_, uri| {
        if let Err(e) = crate::core::package::open_url(uri) {
            log::error!("Failed to open URL {}: {}", uri, e);
        }
        glib::Propagation::Stop
    });

    let dialog_clone = dialog.clone();
    cancel_button.connect_clicked(move |_| {
        info!("Confirmation dialog cancelled");
        dialog_clone.close();
    });

    let dialog_clone = dialog.clone();
    let on_confirm_rc = Rc::new(RefCell::new(Some(on_confirm)));

    continue_button.connect_clicked(move |_| {
        info!("Confirmation dialog confirmed");
        if let Some(on_confirm) = on_confirm_rc.borrow_mut().take() {
            on_confirm();
        }
        dialog_clone.close();
    });

    dialog.present();
}
```

Register it in `gui/src/ui/dialogs/mod.rs`: add `pub mod confirm;`
alongside the other `pub mod` lines.

Add a new module for SecureBoot state detection, `gui/src/core/secureboot.rs`:

```rust
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_secureboot_enabled_does_not_panic() {
        let _ = is_secureboot_enabled();
    }
}
```

Register it in `gui/src/core/mod.rs`: add `pub mod secureboot;` alongside
the other `pub mod` lines, and `pub use secureboot::is_secureboot_enabled;`
alongside the other re-exports.

- [ ] **Step 3: Write the handler**

In `gui/src/ui/pages/servicing.rs`, add these imports alongside the
existing ones near the top of the file:

```rust
use gtk4::glib;
use log::error;
use std::process::{Command as StdCommand, Stdio};
```

(The file already imports `log::info` - change that line to
`use log::{error, info};` instead of adding a duplicate `log` import.)

Add the call in `setup_handlers`, after `setup_switch_to_limine`:

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
    setup_secureboot_setup(page_builder, window);
}
```

Add the new handler function at the end of the file:

```rust
fn setup_secureboot_setup(page_builder: &Builder, window: &ApplicationWindow) {
    let btn_secureboot_setup =
        extract_widget::<gtk4::Button>(page_builder, "btn_secureboot_setup");

    btn_secureboot_setup
        .set_visible(!core::is_package_installed("limine") && !core::is_secureboot_enabled());

    use crate::ui::dialogs::confirm::show_confirmation;

    let window = window.clone();
    btn_secureboot_setup.connect_clicked(move |_| {
        info!("Servicing: SecureBoot Setup button clicked");

        let window_clone = window.clone();
        show_confirmation(
            window.upcast_ref(),
            "Are you sure?",
            "This will install xsb-gui if needed, then minimize xero-toolkit and launch it to set up SecureBoot.",
            move || {
                if core::is_package_installed("xsb-gui") {
                    launch_xsb_gui_and_minimize(&window_clone);
                    return;
                }

                let commands = CommandSequence::new()
                    .then(
                        Command::builder()
                            .aur()
                            .args(&["-S", "--noconfirm", "--needed", "xsb-gui"])
                            .description("Installing SecureBoot Setup tool...")
                            .build(),
                    )
                    .build();

                task_runner::run(window_clone.upcast_ref(), commands, "SecureBoot Setup");

                let window_clone2 = window_clone.clone();
                glib::timeout_add_seconds_local(1, move || {
                    if task_runner::is_running() {
                        return glib::ControlFlow::Continue;
                    }
                    if core::is_package_installed("xsb-gui") {
                        launch_xsb_gui_and_minimize(&window_clone2);
                    }
                    glib::ControlFlow::Break
                });
            },
        );
    });
}

fn launch_xsb_gui_and_minimize(window: &ApplicationWindow) {
    info!("Launching xsb-gui...");
    if let Err(e) = StdCommand::new("xsb-gui")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        error!("Failed to launch xsb-gui: {}", e);
        return;
    }
    window.minimize();
}
```

Also update the module doc-comment at the top of the file to add
"- SecureBoot setup (xsb-gui)" to the list ending in "Switch GRUB to
Limine".

- [ ] **Step 4: Build**

Run: `cargo build -p xero-toolkit`
Expected: compiles with no errors or warnings.

- [ ] **Step 5: Lint and format**

Run: `cargo clippy -p xero-toolkit -- -D warnings` and `cargo fmt --check`
Expected: both clean. If `cargo fmt` reformats, run `cargo fmt` (not
`--check`) to apply, then re-verify the build still passes.

- [ ] **Step 6: Manual verification**

Launch `./target/debug/xero-toolkit`, open Servicing/System Tweaks:
- On this GRUB-based machine, confirm "SecureBoot Setup" is visible next
  to "Switch GRUB to Limine".
- Click it: the plain "Are you sure?" popup appears (no warning icon, no
  "Warning" window title). Cancel it: nothing happens.
- Click again and Continue. If `xsb-gui` is not installed: progress
  dialog appears, runs the AUR install, then the toolkit window minimizes
  and an `xsb-gui` window appears. If already installed: no progress
  dialog, window minimizes and `xsb-gui` launches immediately.
- Cancel the install (if prompted by the AUR helper) or force a failure
  after confirming: the toolkit window does NOT minimize and nothing
  crashes.

- [ ] **Step 7: Commit**

Per standing project rule, do not run `git commit`/`git push` - leave
these changes staged in the working tree for the user to commit
themselves.
