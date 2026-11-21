//! Servicing and system tweaks page button handlers.
//!
//! Handles:
//! - Clear Pacman cache
//! - Unlock Pacman database
//! - Pacman local database fix
//! - Fix GPGME database
//! - Fix Arch keyring
//! - Update mirrorlist
//! - Parallel downloads adjustment
//! - Switch GRUB to Limine
//! - SecureBoot guide (XeroLinux wiki)

use crate::core;
use crate::ui::dialogs::selection::{
    show_selection_dialog, SelectionDialogConfig, SelectionOption, SelectionType,
};
use crate::ui::dialogs::terminal;
use crate::ui::task_runner::{self, Command, CommandSequence};
use crate::ui::utils::extract_widget;
use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Builder};
use log::{error, info};

const SECUREBOOT_GUIDE_URL: &str = "https://wiki.xerolinux.xyz/secure-boot/";

/// Set up all button handlers for the servicing/system tweaks page
pub fn setup_handlers(page_builder: &Builder, _main_builder: &Builder, window: &ApplicationWindow) {
    crate::ui::dialogs::button_info::attach_to_builder(page_builder, window.upcast_ref());
    setup_clr_pacman(page_builder, window);
    setup_unlock_pacman(page_builder, window);
    setup_pacman_db_fix(page_builder, window);
    setup_fix_gpgme(page_builder, window);
    setup_fix_arch_keyring(page_builder, window);
    setup_update_mirrorlist(page_builder, window);
    setup_parallel_downloads(page_builder, window);
    setup_switch_to_limine(page_builder, window);
    setup_secureboot_guide(page_builder);
}

fn setup_clr_pacman(page_builder: &Builder, window: &ApplicationWindow) {
    let btn_clr_pacman = extract_widget::<gtk4::Button>(page_builder, "btn_clr_pacman");
    let window = window.clone();
    btn_clr_pacman.connect_clicked(move |_| {
        info!("Servicing: Clear Pacman Cache button clicked");
        // Use terminal dialog for interactive pacman cache clearing
        terminal::show_terminal_dialog(
            window.upcast_ref(),
            "Clear Pacman Cache",
            "pkexec",
            &["pacman", "-Scc"],
            false,
        );
    });
}

fn setup_unlock_pacman(page_builder: &Builder, window: &ApplicationWindow) {
    let btn_unlock_pacman = extract_widget::<gtk4::Button>(page_builder, "btn_unlock_pacman");
    let window = window.clone();
    btn_unlock_pacman.connect_clicked(move |_| {
        info!("Servicing: Unlock Pacman DB button clicked");
        let commands = CommandSequence::new()
            .then(
                Command::builder()
                    .privileged()
                    .program("rm")
                    .args(&["-f", "/var/lib/pacman/db.lck"])
                    .description("Removing Pacman lock file...")
                    .build(),
            )
            .build();
        task_runner::run(window.upcast_ref(), commands, "Unlock Pacman Database");
    });
}

fn setup_pacman_db_fix(page_builder: &Builder, window: &ApplicationWindow) {
    let btn_pacman_db_fix = extract_widget::<gtk4::Button>(page_builder, "btn_pacman_db_fix");
    let window = window.clone();
    btn_pacman_db_fix.connect_clicked(move |_| {
        info!("Servicing: Pacman DB Fix button clicked");
        let commands = CommandSequence::new()
            .then(
                Command::builder()
                    .privileged()
                    .program("sh")
                    .args(&[
                        "-c",
                        "find /var/lib/pacman/local/ -type f -name 'desc' -exec sed -i '/^%INSTALLED_DB%$/,+2d' {} \\;",
                    ])
                    .description("Fixing Pacman local database...")
                    .build(),
            )
            .build();
        task_runner::run(window.upcast_ref(), commands, "Pacman DB Fix");
    });
}

fn setup_fix_gpgme(page_builder: &Builder, window: &ApplicationWindow) {
    let btn_fix_gpgme = extract_widget::<gtk4::Button>(page_builder, "btn_fix_gpgme");
    let window = window.clone();
    btn_fix_gpgme.connect_clicked(move |_| {
        info!("Servicing: Fix GPGME Database button clicked");
        // Use terminal dialog for interactive GPGME fix
        terminal::show_terminal_dialog(
            window.upcast_ref(),
            "Fix GPGME Database",
            "pkexec",
            &["sh", "-c", "rm -rf /var/lib/pacman/sync && pacman -Syy"],
            false,
        );
    });
}

fn setup_fix_arch_keyring(page_builder: &Builder, window: &ApplicationWindow) {
    let btn_fix_arch_keyring = extract_widget::<gtk4::Button>(page_builder, "btn_fix_arch_keyring");
    let window = window.clone();
    btn_fix_arch_keyring.connect_clicked(move |_| {
        info!("Servicing: Fix Arch Keyring button clicked");
        let commands = CommandSequence::new()
            .then(Command::builder()
                .privileged()
                .program("rm")
                .args(&["-rf", "/etc/pacman.d/gnupg"])
                .description("Removing existing GnuPG keyring...")
                .build())
            .then(Command::builder()
                .privileged()
                .program("pacman-key")
                .args(&["--init"])
                .description("Initializing new keyring...")
                .build())
            .then(Command::builder()
                .privileged()
                .program("pacman-key")
                .args(&["--populate"])
                .description("Populating keyring...")
                .build())
            .then(Command::builder()
                .privileged()
                .program("sh")
                .args(&["-c", "echo 'keyserver hkp://keyserver.ubuntu.com:80' >> /etc/pacman.d/gnupg/gpg.conf"])
                .description("Setting keyserver...")
                .build())
            .then(Command::builder()
                .privileged()
                .program("pacman")
                .args(&["-Syy", "--noconfirm", "archlinux-keyring"])
                .description("Reinstalling Arch Linux keyring...")
                .build())
            .build();
        task_runner::run(window.upcast_ref(), commands, "Fix GnuPG Keyring");
    });
}

fn setup_update_mirrorlist(page_builder: &Builder, window: &ApplicationWindow) {
    let btn_update_mirrorlist =
        extract_widget::<gtk4::Button>(page_builder, "btn_update_mirrorlist");
    let window = window.clone();
    btn_update_mirrorlist.connect_clicked(move |_| {
        info!("Servicing: Update Mirrorlist button clicked");
        let window_ref = window.upcast_ref();

                let rate_mirrors_installed = core::is_package_installed("rate-mirrors");
                let config = SelectionDialogConfig::new(
                    "Update Mirrorlist",
                    "Select which mirrorlists to update. rate-mirrors will be installed if needed.",
                )
                .selection_type(SelectionType::Single)
                .selection_required(false)
                .add_option(SelectionOption::new(
                    "chaotic",
                    "Chaotic-AUR Mirrorlist",
                    "Also update Chaotic-AUR mirrorlist (optional)",
                    false,
                ))
                .confirm_label("Update");

                let window_for_closure = window.clone();
                show_selection_dialog(window_ref, config, move |selected_ids| {
                    let mut commands = CommandSequence::new();

                    if !rate_mirrors_installed {
                        commands = commands.then(Command::builder()
                             .aur()
                             .args(&["-S", "--needed", "--noconfirm", "rate-mirrors"])
                             .description("Installing rate-mirrors utility...")
                             .build());
                    }

                    commands = commands.then(Command::builder()
                        .privileged()
                        .program("sh")
                        .args(&["-c", "rate-mirrors --allow-root --protocol https arch | tee /etc/pacman.d/mirrorlist"])
                        .description("Updating Arch mirrorlist...")
                        .build());

                    if selected_ids.iter().any(|s| s == "chaotic") {
                        commands = commands.then(Command::builder()
                            .privileged()
                            .program("sh")
                            .args(&["-c", "rate-mirrors --allow-root --protocol https chaotic-aur | tee /etc/pacman.d/chaotic-mirrorlist"])
                            .description("Updating Chaotic-AUR mirrorlist...")
                            .build());
                    }

                    if !commands.is_empty() {
                        task_runner::run(window_for_closure.upcast_ref(), commands.build(), "Update System Mirrorlist");
                    }
                });
    });
}

fn setup_parallel_downloads(page_builder: &Builder, window: &ApplicationWindow) {
    let btn_parallel_downloads =
        extract_widget::<gtk4::Button>(page_builder, "btn_parallel_downloads");
    let window = window.clone();
    btn_parallel_downloads.connect_clicked(move |_| {
        info!("Servicing: Change Parallel Downloads button clicked");
        // Use terminal dialog for interactive pmpd tool
        terminal::show_terminal_dialog(
            window.upcast_ref(),
            "Change Parallel Downloads",
            "pkexec",
            &["pmpd"],
            false,
        );
    });
}

fn setup_switch_to_limine(page_builder: &Builder, window: &ApplicationWindow) {
    use crate::ui::dialogs::warning::show_warning_confirmation_styled;

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
        show_warning_confirmation_styled(
            window.upcast_ref(),
            "Switch GRUB to Limine",
            "This will replace Grub with Limine Bootloader. There ain't no coming back from \
            that. Are you sure you want to proceed?",
            true,
            move || {
                let script = crate::config::paths::scripts()
                    .join("switch_grub_to_limine.sh")
                    .to_string_lossy()
                    .into_owned();

                let chainload_script = crate::config::paths::scripts()
                    .join("apply_limine_chainload.sh")
                    .to_string_lossy()
                    .into_owned();

                let wallpaper_script = crate::config::paths::scripts()
                    .join("apply_limine_wallpaper.sh")
                    .to_string_lossy()
                    .into_owned();

                let cleanup_script = crate::config::paths::scripts()
                    .join("cleanup_limine_esp.sh")
                    .to_string_lossy()
                    .into_owned();

                let commands = CommandSequence::new()
                    .then(
                        Command::builder()
                            .aur()
                            .args(&[
                                "-S",
                                "--noconfirm",
                                "--needed",
                                "limine",
                                "limine-mkinitcpio-hook",
                            ])
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
                            .program("/usr/bin/mkinitcpio")
                            .args(&["-P"])
                            .description("Rebuilding initramfs...")
                            .build(),
                    )
                    .then(
                        Command::builder()
                            .privileged()
                            .program("limine-mkinitcpio")
                            .description("Generating Limine boot entries...")
                            .build(),
                    )
                    .then(
                        Command::builder()
                            .privileged()
                            .program("bash")
                            .args(&[&chainload_script])
                            .description("Adding chainload entries for other detected OSes...")
                            .build(),
                    )
                    .then(
                        Command::builder()
                            .privileged()
                            .program("bash")
                            .args(&[&wallpaper_script])
                            .description("Applying XeroLinux boot splash wallpaper...")
                            .build(),
                    )
                    .then(
                        Command::builder()
                            .privileged()
                            .program("bash")
                            .args(&[&cleanup_script])
                            .description("Cleaning up EFI system partition...")
                            .build(),
                    )
                    .build();

                task_runner::run(window_clone.upcast_ref(), commands, "Switch GRUB to Limine");
            },
        );
    });
}

fn setup_secureboot_guide(page_builder: &Builder) {
    extract_widget::<gtk4::Button>(page_builder, "btn_secureboot_guide").connect_clicked(|_| {
        info!("Servicing: SecureBoot Guide button clicked");
        if let Err(e) = core::package::open_url(SECUREBOOT_GUIDE_URL) {
            error!("Failed to open SecureBoot guide: {}", e);
        }
    });
}
