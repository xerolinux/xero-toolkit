//! A.I tools page button handlers.
//!
//! Handles:
//! - LM Studio (official AppImage)
//! - Claude (CLI and/or Desktop)
//! - Alpaca (Flatpak)

use crate::config;
use crate::ui::dialogs::selection::{
    show_selection_dialog, SelectionDialogConfig, SelectionOption, SelectionType,
};
use crate::ui::task_runner::{self, Command, CommandSequence};
use crate::ui::utils::extract_widget;
use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Builder, Button};
use log::info;
use std::path::{Path, PathBuf};

const LM_STUDIO_URL: &str = "https://lmstudio.ai/download/bionic/latest/linux/x64";

// Args: url, appimage path, icon path, .desktop path, .desktop contents.
// The icon is optional: a missing PNG only costs the menu entry its icon.
const LM_STUDIO_INSTALL_SCRIPT: &str = r#"set -euo pipefail
url=$1 appimage=$2 icon=$3 desktop=$4 entry=$5
mkdir -p "$(dirname "$appimage")" "$(dirname "$desktop")"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp" "$appimage.part"' EXIT
curl -fL --retry 3 --progress-bar -o "$appimage.part" "$url"
mv -f "$appimage.part" "$appimage"
chmod +x "$appimage"
if (cd "$tmp" && "$appimage" --appimage-extract '*.png' >/dev/null 2>&1); then
  found=$(find "$tmp/squashfs-root" -maxdepth 1 -name '*.png' | head -n 1)
  if [ -n "$found" ]; then cp -fL "$found" "$icon"; fi
fi
printf '%s\n' "$entry" > "$desktop"
if command -v update-desktop-database >/dev/null; then
  update-desktop-database "$(dirname "$desktop")" || true
fi
"#;

struct LmStudioPaths {
    appimage: PathBuf,
    icon: PathBuf,
    desktop: PathBuf,
}

fn lm_studio_paths(home: &Path) -> LmStudioPaths {
    let appimages = home.join(".local/share/xpm/appimages");
    LmStudioPaths {
        appimage: appimages.join("LM-Studio.AppImage"),
        icon: appimages.join("lm-studio.png"),
        desktop: home.join(".local/share/applications/lm-studio.desktop"),
    }
}

fn lm_studio_desktop_entry(paths: &LmStudioPaths) -> String {
    format!(
        "[Desktop Entry]\n\
         Name=LM Studio\n\
         Comment=Discover, download and run local LLMs\n\
         Exec=\"{}\" %U\n\
         Icon={}\n\
         Terminal=false\n\
         Type=Application\n\
         Categories=Development;Utility;",
        paths.appimage.display(),
        paths.icon.display()
    )
}

fn lm_studio_install_args(url: &str, paths: &LmStudioPaths, entry: &str) -> Vec<String> {
    vec![
        "-c".into(),
        LM_STUDIO_INSTALL_SCRIPT.into(),
        "bash".into(),
        url.into(),
        paths.appimage.display().to_string(),
        paths.icon.display().to_string(),
        paths.desktop.display().to_string(),
        entry.into(),
    ]
}

/// Set up all button handlers for the A.I tools page.
pub fn setup_handlers(page_builder: &Builder, _main_builder: &Builder, window: &ApplicationWindow) {
    crate::ui::dialogs::button_info::attach_to_builder(page_builder, window.upcast_ref());
    setup_lm_studio(page_builder, window);
    setup_claude(page_builder, window);
    setup_alpaca(page_builder, window);
}

fn setup_lm_studio(builder: &Builder, window: &ApplicationWindow) {
    let button = extract_widget::<Button>(builder, "btn_lm_studio");
    let window = window.clone();

    button.connect_clicked(move |_| {
        info!("LM Studio button clicked");

        let paths = lm_studio_paths(Path::new(&config::env::get().home));
        let entry = lm_studio_desktop_entry(&paths);
        let args = lm_studio_install_args(LM_STUDIO_URL, &paths, &entry);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();

        let commands = CommandSequence::new()
            .then(
                Command::builder()
                    .normal()
                    .program("bash")
                    .args(&args)
                    .description("Downloading LM Studio AppImage and adding it to the app menu...")
                    .build(),
            )
            .build();

        task_runner::run(window.upcast_ref(), commands, "LM Studio Installation");
    });
}

fn setup_claude(builder: &Builder, window: &ApplicationWindow) {
    let button = extract_widget::<Button>(builder, "btn_claude");
    let window = window.clone();

    button.connect_clicked(move |_| {
        info!("Claude button clicked");

        let dialog_config = SelectionDialogConfig::new(
            "Claude",
            "Select which Claude apps to install.",
        )
        .selection_type(SelectionType::Multi)
        .selection_required(true)
        .add_option(SelectionOption::new(
            "claude_cli",
            "Claude Code (CLI)",
            "Terminal coding agent, installed with the official install script",
            false,
        ))
        .add_option(SelectionOption::new(
            "claude_desktop",
            "Claude Desktop",
            "Desktop chat app (claude-desktop from Chaotic-AUR)",
            false,
        ))
        .confirm_label("Install");

        let window_for_closure = window.clone();
        show_selection_dialog(window.upcast_ref(), dialog_config, move |selected| {
            let mut commands = CommandSequence::new();

            if selected.iter().any(|s| s == "claude_cli") {
                commands = commands.then(
                    Command::builder()
                        .normal()
                        .program("bash")
                        .args(&["-c", "curl -fsSL https://claude.ai/install.sh | bash"])
                        .description("Installing Claude Code CLI...")
                        .build(),
                );
            }

            if selected.iter().any(|s| s == "claude_desktop") {
                commands = commands.then(
                    Command::builder()
                        .aur()
                        .args(&["-S", "--noconfirm", "--needed", "claude-desktop"])
                        .description("Installing Claude Desktop...")
                        .build(),
                );
            }

            if !commands.is_empty() {
                task_runner::run(
                    window_for_closure.upcast_ref(),
                    commands.build(),
                    "Claude Installation",
                );
            }
        });
    });
}

fn setup_alpaca(builder: &Builder, window: &ApplicationWindow) {
    let button = extract_widget::<Button>(builder, "btn_alpaca");
    let window = window.clone();

    button.connect_clicked(move |_| {
        info!("Alpaca button clicked");

        let commands = CommandSequence::new()
            .then(
                Command::builder()
                    .normal()
                    .program("flatpak")
                    .args(&["install", "-y", "flathub", "com.jeffser.Alpaca"])
                    .description("Installing Alpaca from Flathub...")
                    .build(),
            )
            .build();

        task_runner::run(window.upcast_ref(), commands, "Alpaca Installation");
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::process::Command as Process;

    #[test]
    fn lm_studio_paths_follow_xpm_layout() {
        let paths = lm_studio_paths(Path::new("/home/u"));
        assert_eq!(
            paths.appimage,
            Path::new("/home/u/.local/share/xpm/appimages/LM-Studio.AppImage")
        );
        assert_eq!(
            paths.icon,
            Path::new("/home/u/.local/share/xpm/appimages/lm-studio.png")
        );
        assert_eq!(
            paths.desktop,
            Path::new("/home/u/.local/share/applications/lm-studio.desktop")
        );
    }

    #[test]
    fn desktop_entry_launches_appimage() {
        let entry = lm_studio_desktop_entry(&lm_studio_paths(Path::new("/home/u")));
        assert!(entry.starts_with("[Desktop Entry]\n"));
        assert!(entry.contains(
            "\nExec=\"/home/u/.local/share/xpm/appimages/LM-Studio.AppImage\" %U\n"
        ));
        assert!(entry.contains("\nIcon=/home/u/.local/share/xpm/appimages/lm-studio.png\n"));
        assert!(entry.contains("\nType=Application\n"));
        assert!(entry.contains("\nName=LM Studio\n"));
    }

    #[test]
    fn install_script_downloads_and_integrates() {
        let root = std::env::temp_dir().join(format!("xero-ai-tools-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let home = root.join("home");
        std::fs::create_dir_all(&home).unwrap();

        let fake = root.join("fake.AppImage");
        std::fs::write(
            &fake,
            "#!/bin/sh\n[ \"$1\" = --appimage-extract ] && mkdir -p squashfs-root && echo png > squashfs-root/lm-studio.png\n",
        )
        .unwrap();

        let paths = lm_studio_paths(&home);
        let entry = lm_studio_desktop_entry(&paths);
        let status = Process::new("bash")
            .args(lm_studio_install_args(
                &format!("file://{}", fake.display()),
                &paths,
                &entry,
            ))
            .status()
            .unwrap();
        assert!(status.success());

        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&paths.appimage).unwrap().permissions().mode();
        assert!(mode & 0o111 != 0, "AppImage must be executable");
        assert_eq!(std::fs::read_to_string(&paths.icon).unwrap(), "png\n");
        assert_eq!(std::fs::read_to_string(&paths.desktop).unwrap(), entry + "\n");
        assert!(!paths.appimage.with_extension("AppImage.part").exists());

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn failed_download_leaves_no_partial_file() {
        let root = std::env::temp_dir().join(format!("xero-ai-tools-fail-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let paths = lm_studio_paths(&root);
        let entry = lm_studio_desktop_entry(&paths);
        let missing = format!("file://{}", root.join("missing.AppImage").display());
        let partial = paths.appimage.with_extension("AppImage.part");
        std::fs::create_dir_all(partial.parent().unwrap()).unwrap();
        std::fs::write(&partial, "interrupted").unwrap();

        let status = Process::new("bash")
            .args(lm_studio_install_args(&missing, &paths, &entry))
            .status()
            .unwrap();
        assert!(!status.success());
        assert!(!partial.exists());
        assert!(!paths.desktop.exists());

        std::fs::remove_dir_all(&root).unwrap();
    }
}
