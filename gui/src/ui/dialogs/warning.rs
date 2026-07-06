//! Warning confirmation dialog for experimental features.

use crate::ui::utils::extract_widget;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Builder, Button, Label, Window};
use log::info;
use std::cell::RefCell;
use std::rc::Rc;

/// Show a warning confirmation dialog with cancel and continue buttons.
/// Calls on_confirm callback if user clicks continue.
pub fn show_warning_confirmation<F>(parent: &Window, heading: &str, message: &str, on_confirm: F)
where
    F: FnOnce() + 'static,
{
    show_warning_confirmation_styled(parent, heading, message, false, on_confirm)
}

/// Same as [`show_warning_confirmation`], but with `emphasize: true` the
/// message is rendered as a centered, red pill-shaped badge instead of the
/// usual plain left-aligned text - for the rare warning that should read as
/// impossible to miss.
pub fn show_warning_confirmation_styled<F>(
    parent: &Window,
    heading: &str,
    message: &str,
    emphasize: bool,
    on_confirm: F,
) where
    F: FnOnce() + 'static,
{
    info!("Showing warning confirmation dialog: {}", heading);

    let builder = Builder::from_resource(crate::config::resources::dialogs::WARNING);
    let dialog: Window = extract_widget(&builder, "warning_dialog");
    dialog.set_transient_for(Some(parent));

    let heading_label: Label = extract_widget(&builder, "dialog_heading");
    let warning_message: Label = extract_widget(&builder, "warning_message");
    let cancel_button: Button = extract_widget(&builder, "cancel_button");
    let continue_button: Button = extract_widget(&builder, "continue_button");

    heading_label.set_label(heading);
    continue_button.set_label("Continue");

    if emphasize {
        warning_message.set_justify(gtk4::Justification::Center);
        warning_message.set_xalign(0.5);
        warning_message.add_css_class("warning-pill");
    }

    warning_message.set_markup(message);
    warning_message.connect_activate_link(|_, uri| {
        if let Err(e) = crate::core::package::open_url(uri) {
            log::error!("Failed to open URL {}: {}", uri, e);
        }
        glib::Propagation::Stop
    });

    let dialog_clone = dialog.clone();
    cancel_button.connect_clicked(move |_| {
        info!("Warning dialog cancelled");
        dialog_clone.close();
    });

    let dialog_clone = dialog.clone();
    let on_confirm_rc = Rc::new(RefCell::new(Some(on_confirm)));

    continue_button.connect_clicked(move |_| {
        info!("Warning dialog confirmed");
        if let Some(on_confirm) = on_confirm_rc.borrow_mut().take() {
            on_confirm();
        }
        dialog_clone.close();
    });

    dialog.present();
}
