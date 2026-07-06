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
