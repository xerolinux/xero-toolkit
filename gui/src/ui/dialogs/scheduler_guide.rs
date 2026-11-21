//! Scheduler and profile guide, loaded from `data/scheduler_guide.toml`.

use crate::ui::utils::extract_widget;
use adw::prelude::*;
use gtk4::{gio, Builder, Window};
use log::error;
use serde::Deserialize;

const GUIDE_RESOURCE: &str = "/xyz/xerolinux/xero-toolkit/data/scheduler_guide.toml";

#[derive(Debug, Deserialize)]
struct Guide {
    scheduler: Vec<GuideEntry>,
    profile: Vec<GuideEntry>,
}

#[derive(Debug, Deserialize)]
struct GuideEntry {
    #[cfg_attr(not(test), allow(dead_code))]
    id: String,
    name: String,
    summary: String,
    best_for: String,
}

fn parse_guide(text: &str) -> Result<Guide, toml::de::Error> {
    toml::from_str(text)
}

fn load_guide() -> Option<Guide> {
    let bytes = gio::resources_lookup_data(GUIDE_RESOURCE, gio::ResourceLookupFlags::NONE)
        .map_err(|e| error!("Failed to load scheduler guide: {}", e))
        .ok()?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|e| error!("Scheduler guide is not UTF-8: {}", e))
        .ok()?;
    parse_guide(text)
        .map_err(|e| error!("Failed to parse scheduler guide: {}", e))
        .ok()
}

fn cell(text: &str, css: &[&str], width_chars: i32) -> gtk4::Label {
    let label = gtk4::Label::builder()
        .label(text)
        .wrap(true)
        .xalign(0.0)
        .valign(gtk4::Align::Start)
        .width_chars(width_chars)
        .max_width_chars(width_chars)
        .build();
    for class in css {
        label.add_css_class(class);
    }
    label
}

fn fill_table(grid: &gtk4::Grid, entries: &[GuideEntry]) {
    for (col, title) in ["Name", "Description", "Best for"].iter().enumerate() {
        grid.attach(&cell(title, &["heading"], 0), col as i32, 0, 1, 1);
    }

    for (i, entry) in entries.iter().enumerate() {
        let row = 2 * i as i32 + 1;
        grid.attach(&gtk4::Separator::new(gtk4::Orientation::Horizontal), 0, row, 3, 1);
        grid.attach(&cell(&entry.name, &["guide-name"], 10), 0, row + 1, 1, 1);
        let summary = cell(&entry.summary, &[], 40);
        summary.set_hexpand(true);
        grid.attach(&summary, 1, row + 1, 1, 1);
        grid.attach(&cell(&entry.best_for, &["dim-label"], 26), 2, row + 1, 1, 1);
    }
}

pub fn show_scheduler_guide(parent: &impl IsA<Window>) {
    let Some(guide) = load_guide() else {
        return;
    };

    let builder = Builder::from_resource(crate::config::resources::dialogs::SCHEDULER_GUIDE);
    let dialog: adw::Window = extract_widget(&builder, "scheduler_guide_dialog");
    fill_table(&extract_widget(&builder, "schedulers_grid"), &guide.scheduler);
    fill_table(&extract_widget(&builder, "profiles_grid"), &guide.profile);

    let escape = gtk4::ShortcutController::new();
    escape.add_shortcut(gtk4::Shortcut::new(
        gtk4::ShortcutTrigger::parse_string("Escape"),
        Some(gtk4::NamedAction::new("window.close")),
    ));
    dialog.add_controller(escape);

    dialog.set_transient_for(Some(parent));
    dialog.present();

    // Wrapped labels finish sizing after the first layout, which leaves the view scrolled down.
    let scrolls: Vec<gtk4::ScrolledWindow> = ["schedulers_scroll", "profiles_scroll"]
        .iter()
        .map(|id| extract_widget(&builder, id))
        .collect();
    gtk4::glib::idle_add_local_once(move || {
        for scroll in scrolls {
            scroll.vadjustment().set_value(0.0);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const GUIDE_TOML: &str = include_str!("../../../resources/data/scheduler_guide.toml");

    #[test]
    fn guide_covers_every_scx_loader_scheduler() {
        let guide = parse_guide(GUIDE_TOML).unwrap();
        let mut ids: Vec<&str> = guide.scheduler.iter().map(|s| s.id.as_str()).collect();
        ids.sort_unstable();
        let mut expected = vec![
            "beerland", "bpfland", "cake", "cosmos", "flash", "flow", "forge", "lavd", "mlfq",
            "pandemonium", "p2dq", "tickless", "rustland", "rusty",
        ];
        expected.sort_unstable();
        assert_eq!(ids, expected);
    }

    #[test]
    fn guide_profiles_match_mode_dropdown() {
        let guide = parse_guide(GUIDE_TOML).unwrap();
        let ids: Vec<&str> = guide.profile.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["auto", "gaming", "lowlatency", "powersave"]);
    }

    #[test]
    fn guide_entries_have_text() {
        let guide = parse_guide(GUIDE_TOML).unwrap();
        for entry in guide.scheduler.iter().chain(&guide.profile) {
            assert!(!entry.name.is_empty() && !entry.summary.is_empty() && !entry.best_for.is_empty());
        }
    }
}
