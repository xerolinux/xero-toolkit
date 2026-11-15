//! Customization page button handlers.
//!
//! Handles:
//! - ZSH All-in-One setup
//! - Save Desktop tool
//! - GRUB theme installation
//! - Plymouth Manager
//! - Update Layan Theme
//! - Config/Rice reset
//! - Punchi Dock Remastered

use crate::ui::dialogs::terminal;
use crate::ui::task_runner::{self, Command, CommandSequence};
use crate::ui::utils::extract_widget;
use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Builder, Button};
use log::info;

const PUNCHI_DOCK_REPO: &str = "https://github.com/PunchiSoft/punchi-dock-remastered.git";

/// Arch build dependencies listed in the Punchi Dock README; its setup script
/// cannot prompt for them without a terminal.
const PUNCHI_DOCK_BUILD_DEPS: &[&str] = &[
    "base-devel",
    "binutils",
    "cmake",
    "extra-cmake-modules",
    "gettext",
    "git",
    "pkgconf",
    "qt6-base",
    "qt6-declarative",
    "qt6-shadertools",
    "kconfig",
    "kcoreaddons",
    "kglobalaccel",
    "ki18n",
    "kjobwidgets",
    "kio",
    "kpackage",
    "kservice",
    "kwindowsystem",
    "libplasma",
    "plasma-workspace",
    "plasma-activities",
    "plasma-activities-stats",
    "libpipewire",
    "unzip",
    "zip",
];

const PUNCHI_DOCK_SETUP_INFO: &str = "Remove current Plasma Dock then add Punchi Dock Remastered\nthrough Plasma's Add Widgets interface. Have fun !";

/// Set up all button handlers for the customization page.
pub fn setup_handlers(page_builder: &Builder, _main_builder: &Builder, window: &ApplicationWindow) {
    crate::ui::dialogs::button_info::attach_to_builder(page_builder, window.upcast_ref());
    setup_zsh_aio(page_builder, window);
    setup_save_desktop(page_builder, window);
    setup_grub_theme(page_builder, window);
    setup_plymouth_manager(page_builder, window);
    setup_layan_patch(page_builder, window);
    setup_config_reset(page_builder, window);
    setup_punchi_dock(page_builder, window);
}

fn setup_zsh_aio(builder: &Builder, window: &ApplicationWindow) {
    let button = extract_widget::<Button>(builder, "btn_zsh_aio");
    let window = window.clone();

    button.connect_clicked(move |_| {
        info!("ZSH AiO button clicked");

        let env = crate::config::env::get();
        let home = env.home.clone();
        let user = env.user.clone();

        let commands = CommandSequence::new()
            .then(Command::builder()
                .aur()
                .args(&[
                    "-S",
                    "--needed",
                    "--noconfirm",
                    "zsh",
                    "grml-zsh-config",
                    "fastfetch",
                ])
                .description("Installing ZSH and dependencies...")
                .build())
            .then(Command::builder()
                .normal()
                .program("sh")
                .args(&[
                    "-c",
                    "curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh | sh -s -- --unattended",
                ])
                .description("Installing Oh My Zsh framework...")
                .build())
            .then(Command::builder()
                .aur()
                .args(&[
                    "-S",
                    "--noconfirm",
                    "--needed",
                    "pacseek",
                    "ttf-meslo-nerd",
                    "siji-git",
                    "otf-unifont",
                    "bdf-unifont",
                    "noto-color-emoji-fontconfig",
                    "xorg-fonts-misc",
                    "ttf-dejavu",
                    "ttf-meslo-nerd-font-powerlevel10k",
                    "noto-fonts-emoji",
                    "powerline-fonts",
                    "oh-my-posh-bin",
                ])
                .description("Installing fonts and terminal enhancements...")
                .build())
            .then(Command::builder()
                .normal()
                .program("git")
                .args(&[
                    "clone",
                    "https://github.com/zsh-users/zsh-completions",
                    &format!("{}/.oh-my-zsh/custom/plugins/zsh-completions", home),
                ])
                .description("Installing ZSH completions plugin...")
                .build())
            .then(Command::builder()
                .normal()
                .program("git")
                .args(&[
                    "clone",
                    "https://github.com/zsh-users/zsh-autosuggestions",
                    &format!("{}/.oh-my-zsh/custom/plugins/zsh-autosuggestions", home),
                ])
                .description("Installing ZSH autosuggestions plugin...")
                .build())
            .then(Command::builder()
                .normal()
                .program("git")
                .args(&[
                    "clone",
                    "https://github.com/zsh-users/zsh-syntax-highlighting.git",
                    &format!("{}/.oh-my-zsh/custom/plugins/zsh-syntax-highlighting", home),
                ])
                .description("Installing ZSH syntax highlighting plugin...")
                .build())
            .then(Command::builder()
                .normal()
                .program("sh")
                .args(&[
                    "-c",
                    &format!(
                        "mv -f {}/.zshrc {}/.zshrc.user 2>/dev/null || true",
                        home, home
                    ),
                ])
                .description("Backing up existing ZSH configuration...")
                .build())
            .then(Command::builder()
                .normal()
                .program("wget")
                .args(&[
                    "-q",
                    "-P",
                    &home,
                    "https://raw.githubusercontent.com/xerolinux/xero-fixes/main/conf/.zshrc",
                ])
                .description("Downloading XeroLinux ZSH configuration...")
                .build())
            .then(Command::builder()
                .normal()
                .program("sh")
                .args(&[
                    "-c",
                    &format!(
                        "sed -i 's|Command=/bin/bash|Command=/bin/zsh|g' {}/.local/share/konsole/XeroLinux.profile 2>/dev/null || true",
                        home
                    ),
                ])
                .description("Updating Konsole profile to use ZSH...")
                .build())
            .then(Command::builder()
                .privileged()
                .program("chsh")
                .args(&[&user, "-s", "/bin/zsh"])
                .description("Setting ZSH as default shell...")
                .build())
            .then(Command::builder()
                .normal()
                .program("sh")
                .args(&["-c", &zshrc_gh_plugin_script(&home)])
                .description("Switching Oh My Zsh github plugin to gh...")
                .build())
            .build();

        task_runner::run(
            window.upcast_ref(),
            commands,
            "ZSH All-in-One Setup",
        );
    });
}

/// Oh My Zsh deprecated the `github` plugin in favour of `gh`.
fn zshrc_gh_plugin_script(home: &str) -> String {
    format!(r"sed -i '/^plugins=(/s/\bgithub\b/gh/' {}/.zshrc", home)
}

fn punchi_dock_script(repo_url: &str) -> String {
    format!(
        r#"set -e
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
git clone --depth 1 '{repo_url}' "$tmp/punchi-dock-remastered"
cd "$tmp/punchi-dock-remastered"
./scripts-user/setup.sh --install --no-restart"#
    )
}

fn setup_punchi_dock(builder: &Builder, window: &ApplicationWindow) {
    let button = extract_widget::<Button>(builder, "btn_punchi_dock");
    let window = window.clone();

    button.connect_clicked(move |_| {
        info!("Punchi Dock Remastered button clicked");

        let mut pacman_args = vec!["-S", "--needed", "--noconfirm"];
        pacman_args.extend_from_slice(PUNCHI_DOCK_BUILD_DEPS);

        let commands = CommandSequence::new()
            .then(
                Command::builder()
                    .privileged()
                    .program("pacman")
                    .args(&pacman_args)
                    .description("Installing Punchi Dock build dependencies...")
                    .build(),
            )
            .then(
                Command::builder()
                    .normal()
                    .program("bash")
                    .args(&["-c", &punchi_dock_script(PUNCHI_DOCK_REPO)])
                    .description("Building and installing Punchi Dock Remastered...")
                    .build(),
            )
            .on_success(PUNCHI_DOCK_SETUP_INFO)
            .build();

        task_runner::run(
            window.upcast_ref(),
            commands,
            "Punchi Dock Remastered Installation",
        );
    });
}

fn setup_save_desktop(builder: &Builder, window: &ApplicationWindow) {
    let button = extract_widget::<Button>(builder, "btn_save_desktop");
    let window = window.clone();

    button.connect_clicked(move |_| {
        info!("Save Desktop Tool button clicked");

        let commands = CommandSequence::new()
            .then(
                Command::builder()
                    .normal()
                    .program("flatpak")
                    .args(&["install", "-y", "io.github.vikdevelop.SaveDesktop"])
                    .description("Installing Save Desktop tool from Flathub...")
                    .build(),
            )
            .build();

        task_runner::run(
            window.upcast_ref(),
            commands,
            "Save Desktop Tool Installation",
        );
    });
}

fn setup_grub_theme(builder: &Builder, window: &ApplicationWindow) {
    let button = extract_widget::<Button>(builder, "btn_grub_theme");
    let window = window.clone();

    button.connect_clicked(move |_| {
        info!("GRUB Theme button clicked");

        let install_command = "cd /tmp && curl -fsSL 'https://xerolinux.xyz/script/grubs/xero-grubs.py' -o xero-grubs.py && setsid -f python3 /tmp/xero-grubs.py > /dev/null 2>&1 < /dev/null";

        terminal::show_terminal_dialog(
            window.upcast_ref(),
            "XeroLinux GRUB Theme Installation",
            "bash",
            &["-c", install_command],
            true,
        );
    });
}

fn setup_plymouth_manager(builder: &Builder, window: &ApplicationWindow) {
    let button = extract_widget::<Button>(builder, "btn_plymouth_manager");
    let window = window.clone();

    button.connect_clicked(move |_| {
        info!("Plymouth Manager button clicked");

        terminal::show_terminal_dialog(
            window.upcast_ref(),
            "Plymouth Manager",
            "/usr/local/bin/xpm",
            &[],
            false,
        );
    });
}

fn setup_layan_patch(builder: &Builder, window: &ApplicationWindow) {
    let button = extract_widget::<Button>(builder, "btn_layan_patch");
    let window = window.clone();

    button.connect_clicked(move |_| {
        info!("Update Layan Theme button clicked");

        let home = crate::config::env::get().home.clone();

        let commands = CommandSequence::new()
            .then(
                Command::builder()
                    .normal()
                    .program("git")
                    .args(&[
                        "clone",
                        "--depth",
                        "1",
                        "https://github.com/vinceliuice/Layan-kde.git",
                        &format!("{}/Layan-kde", home),
                    ])
                    .description("Downloading Layan KDE theme...")
                    .build(),
            )
            .then(
                Command::builder()
                    .privileged()
                    .program("sh")
                    .args(&["-c", &format!("cd {}/Layan-kde && sh install.sh", home)])
                    .description("Installing Layan KDE theme...")
                    .build(),
            )
            .then(
                Command::builder()
                    .normal()
                    .program("rm")
                    .args(&["-rf", &format!("{}/Layan-kde", home)])
                    .description("Cleaning up KDE theme files...")
                    .build(),
            )
            .build();

        task_runner::run(window.upcast_ref(), commands, "Update Layan Theme");
    });
}

fn setup_config_reset(builder: &Builder, window: &ApplicationWindow) {
    let button = extract_widget::<Button>(builder, "btn_config_reset");
    let window = window.clone();

    button.connect_clicked(move |_| {
        info!("Config/Rice Reset button clicked");

        let window_clone = window.clone();
        crate::ui::dialogs::warning::show_warning_confirmation(
            window.upcast_ref(),
            "Config/Rice Reset",
            "A backup of <span foreground=\"cyan\" weight=\"bold\">~/.config</span> will be created.\n\
             Once reset, the system will <span foreground=\"red\" weight=\"bold\">reboot</span>.\n\n\
             You will be getting updated config as of reset time.",
            move || {
                let commands = CommandSequence::new()
                    .then(
                        Command::builder()
                            .normal()
                            .program("bash")
                            .args(&[
                                "-c",
                                "cp -Rf ~/.config ~/.config-backup-$(date +%Y.%m.%d-%H.%M.%S)",
                            ])
                            .description("Backing up configuration...")
                            .build(),
                    )
                    .then(
                        Command::builder()
                            .normal()
                            .program("bash")
                            .args(&["-c", "cp -Rf /etc/skel/. ~"])
                            .description("Restoring default configuration...")
                            .build(),
                    )
                    .then(
                        Command::builder()
                            .normal()
                            .program("reboot")
                            .description("Rebooting system...")
                            .build(),
                    )
                    .build();

                task_runner::run(
                    window_clone.upcast_ref(),
                    commands,
                    "Config/Rice Reset",
                );
            },
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_punchi_repo(root: &std::path::Path, setup_body: &str) -> String {
        let repo = root.join("repo");
        std::fs::create_dir_all(repo.join("scripts-user")).unwrap();
        let setup = repo.join("scripts-user/setup.sh");
        std::fs::write(&setup, format!("#!/bin/sh\n{setup_body}\n")).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&setup, std::fs::Permissions::from_mode(0o755)).unwrap();
        let git = |args: &[&str]| {
            assert!(std::process::Command::new("git")
                .args(["-C", repo.to_str().unwrap(), "-c", "user.name=t", "-c", "user.email=t@t"])
                .args(args)
                .output()
                .unwrap()
                .status
                .success());
        };
        git(&["init", "-q"]);
        git(&["add", "."]);
        git(&["commit", "-qm", "init"]);
        format!("file://{}", repo.display())
    }

    #[test]
    fn punchi_dock_script_runs_setup_and_removes_clone() {
        let root = std::env::temp_dir().join(format!("xero-punchi-ok-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let marker = root.join("marker");
        let url = fake_punchi_repo(&root, &format!("echo \"$PWD $*\" > {}", marker.display()));

        let status = std::process::Command::new("bash")
            .args(["-c", &punchi_dock_script(&url)])
            .status()
            .unwrap();

        assert!(status.success());
        let recorded = std::fs::read_to_string(&marker).unwrap();
        let (clone_dir, args) = recorded.trim().split_once(' ').unwrap();
        assert_eq!(args, "--install --no-restart");
        assert!(clone_dir.ends_with("/punchi-dock-remastered"));
        assert!(!std::path::Path::new(clone_dir).exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn punchi_dock_script_fails_and_cleans_up_when_setup_fails() {
        let root = std::env::temp_dir().join(format!("xero-punchi-fail-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let marker = root.join("marker");
        let url = fake_punchi_repo(&root, &format!("echo \"$PWD\" > {}; exit 3", marker.display()));

        let status = std::process::Command::new("bash")
            .args(["-c", &punchi_dock_script(&url)])
            .status()
            .unwrap();

        assert!(!status.success());
        let clone_dir = std::fs::read_to_string(&marker).unwrap();
        assert!(!std::path::Path::new(clone_dir.trim()).exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn zshrc_gh_plugin_swaps_only_github_plugin() {
        let dir = std::env::temp_dir().join(format!("xero-zshrc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let zshrc = dir.join(".zshrc");
        std::fs::write(
            &zshrc,
            "# github comment stays\nplugins=(git emoji github jump)\nZSH=github\n",
        )
        .unwrap();

        let status = std::process::Command::new("sh")
            .args(["-c", &zshrc_gh_plugin_script(dir.to_str().unwrap())])
            .status()
            .unwrap();

        assert!(status.success());
        assert_eq!(
            std::fs::read_to_string(&zshrc).unwrap(),
            "# github comment stays\nplugins=(git emoji gh jump)\nZSH=github\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
