//! Miscellaneous XeroLinux tools page button handlers.
//!
//! Handles:
//! - EFI Boot Manager GUI
//! - Wallpaper Browser GUI
//! - PHP Server GUI
//! - XSpeedTest

use crate::ui::task_runner::{self, Command, CommandSequence};
use crate::ui::utils::extract_widget;
use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Builder, Button};
use log::info;

/// Set up all button handlers for the misc tools page.
pub fn setup_handlers(page_builder: &Builder, _main_builder: &Builder, window: &ApplicationWindow) {
    crate::ui::dialogs::button_info::attach_to_builder(page_builder, window.upcast_ref());
    setup_efiboot_manager(page_builder, window);
    setup_wallpaper_browser(page_builder, window);
    setup_php_server_gui(page_builder, window);
    setup_xspeedtest(page_builder, window);
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
