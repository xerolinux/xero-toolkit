//! Miscellaneous XeroLinux tools page button handlers.
//!
//! Handles:
//! - ArtCraft Launcher (latest GitHub AppImage)
//! - EFI Boot Manager GUI
//! - Wallpaper Browser GUI
//! - PHP Server GUI
//! - XSpeedTest

use crate::ui::task_runner::{self, Command, CommandSequence};
use crate::ui::utils::extract_widget;
use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Builder, Button};
use log::info;
use std::path::{Path, PathBuf};

const ARTCRAFT_RELEASE_API: &str =
    "https://api.github.com/repos/storytold/craft-launcher/releases/latest";

// Args: release API URL, AppImage path, icon path, .desktop path, .desktop contents.
// xPackage Manager picks the AppImage up from its folder and handles updates.
const ARTCRAFT_INSTALL_SCRIPT: &str = r#"set -euo pipefail
api=$1 appimage=$2 icon=$3 desktop=$4 entry=$5
tmp=$(mktemp -d)
trap 'rm -rf "$tmp" "$appimage.part"' EXIT
url=$(curl -fsSL "$api" \
  | grep -oE '"browser_download_url": *"[^"]*/artcraft-launcher-[^/"]*-linux-x86_64\.AppImage"' \
  | head -n 1 | sed -E 's/.*"([^"]+)"$/\1/' || true)
if [ -z "$url" ]; then
  echo "No x86_64 AppImage found in the latest ArtCraft Launcher release" >&2
  exit 1
fi
name=${url##*/}
echo "Downloading $name"
curl -fL --retry 3 --progress-bar -o "$tmp/$name" "$url"
curl -fsSL -o "$tmp/SHA256SUMS.txt" "${url%/*}/SHA256SUMS.txt"
sum=$(awk -v n="$name" '$2 == n || $2 == "*" n' "$tmp/SHA256SUMS.txt")
if [ -z "$sum" ]; then
  echo "No checksum listed for $name" >&2
  exit 1
fi
(cd "$tmp" && printf '%s\n' "$sum" | sha256sum -c -)
mkdir -p "$(dirname "$appimage")" "$(dirname "$icon")" "$(dirname "$desktop")"
cp -f "$tmp/$name" "$appimage.part"
chmod +x "$appimage.part"
mv -f "$appimage.part" "$appimage"
if (cd "$tmp" && "$appimage" --appimage-extract '*.png' >/dev/null 2>&1); then
  found=$(find "$tmp/squashfs-root" -maxdepth 1 -name '*.png' | head -n 1)
  if [ -n "$found" ]; then cp -fL "$found" "$icon"; fi
fi
printf '%s\n' "$entry" > "$desktop"
if command -v update-desktop-database >/dev/null; then
  update-desktop-database "$(dirname "$desktop")" || true
fi
"#;

struct ArtCraftPaths {
    appimage: PathBuf,
    icon: PathBuf,
    desktop: PathBuf,
}

fn artcraft_paths(home: &Path) -> ArtCraftPaths {
    let share = home.join(".local/share");
    ArtCraftPaths {
        appimage: share.join("xpm/appimages/artcraft-launcher.appimage"),
        icon: share.join("icons/xpm-appimage-artcraft-launcher.png"),
        desktop: share.join("applications/xpm-appimage-artcraft-launcher.desktop"),
    }
}

fn artcraft_desktop_entry(paths: &ArtCraftPaths) -> String {
    let app = paths.appimage.display();
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=ArtCraft Launcher\n\
         Comment=Install, update and launch the Craft App Suite\n\
         Exec=\"{app}\" %U\n\
         TryExec={app}\n\
         Icon={}\n\
         Categories=Utility;\n\
         Terminal=false\n\
         X-XPM-AppImage=true",
        paths.icon.display()
    )
}

fn artcraft_install_args(api: &str, paths: &ArtCraftPaths, entry: &str) -> Vec<String> {
    vec![
        "-c".into(),
        ARTCRAFT_INSTALL_SCRIPT.into(),
        "bash".into(),
        api.into(),
        paths.appimage.display().to_string(),
        paths.icon.display().to_string(),
        paths.desktop.display().to_string(),
        entry.into(),
    ]
}

/// Set up all button handlers for the misc tools page.
pub fn setup_handlers(page_builder: &Builder, _main_builder: &Builder, window: &ApplicationWindow) {
    crate::ui::dialogs::button_info::attach_to_builder(page_builder, window.upcast_ref());
    setup_artcraft_launcher(page_builder, window);
    setup_efiboot_manager(page_builder, window);
    setup_wallpaper_browser(page_builder, window);
    setup_php_server_gui(page_builder, window);
    setup_xspeedtest(page_builder, window);
}

fn setup_artcraft_launcher(builder: &Builder, window: &ApplicationWindow) {
    let button = extract_widget::<Button>(builder, "btn_artcraft_launcher");
    let window = window.clone();

    button.connect_clicked(move |_| {
        info!("ArtCraft Launcher button clicked");

        let paths = artcraft_paths(Path::new(&crate::config::env::get().home));
        let entry = artcraft_desktop_entry(&paths);
        let args = artcraft_install_args(ARTCRAFT_RELEASE_API, &paths, &entry);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();

        let commands = CommandSequence::new()
            .then(
                Command::builder()
                    .normal()
                    .program("bash")
                    .args(&args)
                    .description("Downloading latest ArtCraft Launcher AppImage...")
                    .build(),
            )
            .build();

        task_runner::run(window.upcast_ref(), commands, "ArtCraft Launcher Installation");
    });
}

fn setup_efiboot_manager(builder: &Builder, window: &ApplicationWindow) {
    let button = extract_widget::<Button>(builder, "btn_efiboot_manager");
    let window = window.clone();

    button.connect_clicked(move |_| {
        info!("EFI Boot Manager button clicked");

        let commands = CommandSequence::new()
            .then(
                Command::builder()
                    .aur()
                    .args(&["-S", "--noconfirm", "--needed", "efibootmgrgui"])
                    .description("Installing EFI Boot Manager GUI from XeroLinux repo...")
                    .build(),
            )
            .build();

        task_runner::run(
            window.upcast_ref(),
            commands,
            "EFI Boot Manager Installation",
        );
    });
}

fn setup_wallpaper_browser(builder: &Builder, window: &ApplicationWindow) {
    let button = extract_widget::<Button>(builder, "btn_wallpaper_browser");
    let window = window.clone();

    button.connect_clicked(move |_| {
        info!("Wallpaper Browser button clicked");

        let commands = CommandSequence::new()
            .then(
                Command::builder()
                    .aur()
                    .args(&["-S", "--noconfirm", "--needed", "xero-wallpaper-browser"])
                    .description("Installing Wallpaper Browser GUI from XeroLinux repo...")
                    .build(),
            )
            .build();

        task_runner::run(
            window.upcast_ref(),
            commands,
            "Wallpaper Browser Installation",
        );
    });
}

fn setup_php_server_gui(builder: &Builder, window: &ApplicationWindow) {
    let button = extract_widget::<Button>(builder, "btn_php_server_gui");
    let window = window.clone();

    button.connect_clicked(move |_| {
        info!("PHP Server GUI button clicked");

        let commands = CommandSequence::new()
            .then(
                Command::builder()
                    .aur()
                    .args(&["-S", "--noconfirm", "--needed", "phpgui"])
                    .description("Installing PHP Server GUI from XeroLinux repo...")
                    .build(),
            )
            .build();

        task_runner::run(window.upcast_ref(), commands, "PHP Server GUI Installation");
    });
}

fn setup_xspeedtest(builder: &Builder, window: &ApplicationWindow) {
    let button = extract_widget::<Button>(builder, "btn_xspeedtest");
    let window = window.clone();

    button.connect_clicked(move |_| {
        info!("XSpeedTest button clicked");

        let commands = CommandSequence::new()
            .then(
                Command::builder()
                    .aur()
                    .args(&["-S", "--noconfirm", "--needed", "xspeedtest"])
                    .description("Installing XSpeedTest from XeroLinux repo...")
                    .build(),
            )
            .build();

        task_runner::run(window.upcast_ref(), commands, "XSpeedTest Installation");
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command as Process;

    struct FakeRelease {
        root: PathBuf,
        api: String,
        paths: ArtCraftPaths,
    }

    fn fake_release(tag: &str, checksum_ok: bool) -> FakeRelease {
        let root = std::env::temp_dir().join(format!("xero-artcraft-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dl = root.join("dl");
        std::fs::create_dir_all(&dl).unwrap();

        let name = "artcraft-launcher-9.9.9-linux-x86_64.AppImage";
        let appimage = dl.join(name);
        std::fs::write(
            &appimage,
            "#!/bin/sh\n[ \"$1\" = --appimage-extract ] && mkdir -p squashfs-root && echo png > squashfs-root/artcraft-launcher.png\n",
        )
        .unwrap();
        let sum = String::from_utf8(
            Process::new("sha256sum").arg(&appimage).output().unwrap().stdout,
        )
        .unwrap();
        let hash = if checksum_ok {
            sum.split_whitespace().next().unwrap().to_string()
        } else {
            "0".repeat(64)
        };
        std::fs::write(dl.join("SHA256SUMS.txt"), format!("{hash}  {name}\n")).unwrap();

        let json = root.join("latest.json");
        std::fs::write(
            &json,
            format!(
                "{{\n  \"assets\": [\n    {{\n      \"browser_download_url\": \"file://{d}/artcraft-launcher-9.9.9-linux-aarch64.AppImage\"\n    }},\n    {{\n      \"browser_download_url\": \"file://{d}/{name}\"\n    }}\n  ]\n}}\n",
                d = dl.display()
            ),
        )
        .unwrap();

        FakeRelease {
            api: format!("file://{}", json.display()),
            paths: artcraft_paths(&root.join("home")),
            root,
        }
    }

    fn run_install(release: &FakeRelease) -> bool {
        let entry = artcraft_desktop_entry(&release.paths);
        Process::new("bash")
            .args(artcraft_install_args(&release.api, &release.paths, &entry))
            .status()
            .unwrap()
            .success()
    }

    #[test]
    fn artcraft_paths_follow_xpm_layout() {
        let paths = artcraft_paths(Path::new("/home/u"));
        assert_eq!(
            paths.appimage,
            Path::new("/home/u/.local/share/xpm/appimages/artcraft-launcher.appimage")
        );
        assert_eq!(
            paths.icon,
            Path::new("/home/u/.local/share/icons/xpm-appimage-artcraft-launcher.png")
        );
        assert_eq!(
            paths.desktop,
            Path::new("/home/u/.local/share/applications/xpm-appimage-artcraft-launcher.desktop")
        );
    }

    #[test]
    fn desktop_entry_is_xpm_appimage_entry() {
        let entry = artcraft_desktop_entry(&artcraft_paths(Path::new("/home/u")));
        let app = "/home/u/.local/share/xpm/appimages/artcraft-launcher.appimage";
        assert!(entry.starts_with("[Desktop Entry]\n"));
        assert!(entry.contains("\nName=ArtCraft Launcher\n"));
        assert!(entry.contains(&format!("\nExec=\"{app}\" %U\n")));
        assert!(entry.contains(&format!("\nTryExec={app}\n")));
        assert!(entry.contains(
            "\nIcon=/home/u/.local/share/icons/xpm-appimage-artcraft-launcher.png\n"
        ));
        assert!(entry.contains("\nX-XPM-AppImage=true"));
    }

    #[test]
    fn install_picks_x86_64_appimage_and_integrates() {
        let release = fake_release("ok", true);
        assert!(run_install(&release));

        let paths = &release.paths;
        let mode = std::fs::metadata(&paths.appimage).unwrap().permissions().mode();
        assert!(mode & 0o111 != 0, "AppImage must be executable");
        assert!(std::fs::read_to_string(&paths.appimage).unwrap().starts_with("#!/bin/sh"));
        assert_eq!(std::fs::read_to_string(&paths.icon).unwrap(), "png\n");
        assert_eq!(
            std::fs::read_to_string(&paths.desktop).unwrap(),
            artcraft_desktop_entry(paths) + "\n"
        );

        std::fs::remove_dir_all(&release.root).unwrap();
    }

    #[test]
    fn checksum_mismatch_installs_nothing() {
        let release = fake_release("bad", false);
        assert!(!run_install(&release));

        let paths = &release.paths;
        assert!(!paths.appimage.exists());
        assert!(!paths.appimage.with_extension("appimage.part").exists());
        assert!(!paths.desktop.exists());

        std::fs::remove_dir_all(&release.root).unwrap();
    }
}
