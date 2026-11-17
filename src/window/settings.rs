use gtk4::{gdk, prelude::*};

use crate::meta::list_monitors;

use super::overlay::OverlayManager;

pub fn build_settings_popover(manager: &OverlayManager) -> gtk4::Popover {
    let popover = gtk4::Popover::new();
    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    content.set_size_request(320, -1);
    content.set_margin_start(16);
    content.set_margin_end(16);
    content.set_margin_top(16);
    content.set_margin_bottom(16);

    let title = gtk4::Label::new(Some("Settings"));
    title.set_halign(gtk4::Align::Start);
    title.add_css_class("title-4");
    content.append(&title);

    let displays_title = gtk4::Label::new(Some("Display"));
    displays_title.set_halign(gtk4::Align::Start);
    displays_title.add_css_class("heading");
    content.append(&displays_title);

    let displays = gtk4::FlowBox::new();
    displays.set_selection_mode(gtk4::SelectionMode::None);
    displays.set_min_children_per_line(2);
    displays.set_max_children_per_line(3);
    displays.set_column_spacing(8);
    displays.set_row_spacing(8);
    content.append(&displays);

    let all_monitors = gtk4::CheckButton::with_label("Cover all monitors");
    all_monitors.set_active(manager.settings().all_monitors);
    let checkbox_manager = manager.clone();
    all_monitors.connect_toggled(move |button| {
        let mut settings = checkbox_manager.settings();
        settings.all_monitors = button.is_active();
        if let Err(error) = checkbox_manager.update_settings(settings) {
            eprintln!("Could not update display settings: {error}");
        }
    });
    content.append(&all_monitors);
    popover.set_child(Some(&content));

    let display_manager = manager.clone();
    let settings_popover = popover.downgrade();
    let displays = displays.downgrade();
    let all_monitors = all_monitors.downgrade();
    popover.connect_show(move |_| {
        let (Some(displays), Some(all_monitors)) = (displays.upgrade(), all_monitors.upgrade())
        else {
            return;
        };
        while let Some(child) = displays.first_child() {
            displays.remove(&child);
        }
        let Some(display) = gdk::Display::default() else {
            return;
        };
        let monitors = list_monitors(&display);
        let saved = display_manager.settings();
        let selected = monitors
            .iter()
            .any(|monitor| monitor.connector.as_str() == saved.connector);
        let mut group: Option<gtk4::ToggleButton> = None;
        for monitor in monitors {
            let connector = monitor.connector.to_string();
            let button = gtk4::ToggleButton::new();
            button.set_size_request(84, 84);
            button.set_tooltip_text(Some(&connector));
            let label = gtk4::Label::new(Some(&connector));
            label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
            label.set_max_width_chars(12);
            button.set_child(Some(&label));
            if let Some(first) = &group {
                button.set_group(Some(first));
            } else {
                group = Some(button.clone());
            }
            if connector == saved.connector || (!selected && monitor.id == 0) {
                button.set_active(true);
            }
            let selected_manager = display_manager.clone();
            let settings_popover = settings_popover.clone();
            button.connect_toggled(move |button| {
                if button.is_active() {
                    let mut settings = selected_manager.settings();
                    settings.connector = connector.clone();
                    if let Some(popover) = settings_popover.upgrade() {
                        popover.popdown();
                    }
                    if let Err(error) = selected_manager.update_settings(settings) {
                        eprintln!("Could not select display: {error}");
                    }
                }
            });
            displays.insert(&button, -1);
        }
        all_monitors.set_active(saved.all_monitors);
    });

    popover
}
