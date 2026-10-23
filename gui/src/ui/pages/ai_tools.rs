//! A.I tools page button handlers.
//!
//! Handles:
//! - LM Studio (Flatpak)
//! - Claude (CLI and/or Desktop)
//! - Alpaca (Flatpak)

use crate::ui::dialogs::selection::{
    show_selection_dialog, SelectionDialogConfig, SelectionOption, SelectionType,
};
use crate::ui::task_runner::{self, Command, CommandSequence};
use crate::ui::utils::extract_widget;
use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Builder, Button};
use log::info;

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

        let commands = CommandSequence::new()
            .then(
                Command::builder()
                    .normal()
                    .program("flatpak")
                    .args(&["install", "-y", "flathub", "ai.lmstudio.lm-studio"])
                    .description("Installing LM Studio from Flathub...")
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
