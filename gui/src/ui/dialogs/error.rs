//! Shared dialog helpers used across the UI pages.

use adw::prelude::*;
use adw::AlertDialog;
use crate::ui::utils::extract_widget;
use gtk4::ApplicationWindow;

/// Show an error message dialog transient for the provided window.
pub fn show_error(window: &ApplicationWindow, message: &str) {
    let dialog = AlertDialog::builder()
        .heading("Error")
        .body(message)
        .build();

    dialog.present(Some(window));
}

/// Show a friendly popup with a smiley, heading, message and OK button.
pub fn show_notice(parent: &impl IsA<gtk4::Window>, heading: &str, message: &str) {
    let builder = gtk4::Builder::from_resource(crate::config::resources::dialogs::NOTICE);
    let dialog: adw::Window = extract_widget(&builder, "notice_dialog");
    let ok_button: gtk4::Button = extract_widget(&builder, "notice_ok_button");

    extract_widget::<gtk4::Label>(&builder, "notice_heading").set_label(heading);
    extract_widget::<gtk4::Label>(&builder, "notice_message").set_label(message);
    dialog.set_transient_for(Some(parent));
    dialog.set_default_widget(Some(&ok_button));

    let dialog_weak = dialog.downgrade();
    ok_button.connect_clicked(move |_| {
        if let Some(dialog) = dialog_weak.upgrade() {
            dialog.close();
        }
    });

    let escape = gtk4::ShortcutController::new();
    escape.add_shortcut(gtk4::Shortcut::new(
        gtk4::ShortcutTrigger::parse_string("Escape"),
        Some(gtk4::NamedAction::new("window.close")),
    ));
    dialog.add_controller(escape);

    dialog.present();
}
