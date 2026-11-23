use gtk4::{gdk, prelude::*};

use crate::meta::list_monitors;

use super::{
    overlay::OverlayManager,
    shortcuts::{Action, Binding, PermissionState, ShortcutManager},
};

pub fn build_settings_popover(
    manager: &OverlayManager,
    shortcuts: &ShortcutManager,
) -> gtk4::Popover {
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

    content.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
    let shortcuts_title = gtk4::Label::new(Some("Global shortcuts"));
    shortcuts_title.set_halign(gtk4::Align::Start);
    shortcuts_title.add_css_class("heading");
    content.append(&shortcuts_title);
    let shortcut_controls = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    let permission_button = if shortcuts.is_wayland() {
        let button = gtk4::Button::with_label("Enable global shortcuts");
        let permission_manager = shortcuts.clone();
        button.connect_clicked(move |_| permission_manager.request_permission());
        content.append(&button);
        Some(button)
    } else {
        None
    };
    let shortcut_status = gtk4::Label::new(None);
    shortcut_status.set_wrap(true);
    shortcut_status.set_halign(gtk4::Align::Start);
    shortcut_status.add_css_class("dim-label");
    let status_for_show = shortcut_status.downgrade();
    let shortcuts_for_show = shortcuts.clone();
    let shortcut_buttons: std::rc::Rc<std::cell::RefCell<Vec<(Action, gtk4::Button)>>> =
        std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let buttons_for_show = shortcut_buttons.clone();
    popover.connect_show(move |_| {
        if let Some(status) = status_for_show.upgrade() {
            let permission = shortcuts_for_show.permission_state();
            let registered_status = shortcuts_for_show.status();
            status.set_label(match &permission {
                PermissionState::NotRequested => "Global shortcuts need desktop approval",
                PermissionState::Requesting => "Waiting for desktop approval...",
                PermissionState::Denied(error) => error,
                PermissionState::Granted => &registered_status,
            });
        }
        for (action, button) in buttons_for_show.borrow().iter() {
            button.set_label(&shortcuts_for_show.display_label(*action));
        }
    });
    for action in Action::ALL {
        let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        let label = gtk4::Label::new(Some(action.label()));
        label.set_halign(gtk4::Align::Start);
        label.set_hexpand(true);
        row.append(&label);

        let capture = gtk4::Button::with_label(&shortcuts.display_label(action));
        shortcut_buttons
            .borrow_mut()
            .push((action, capture.clone()));
        capture.set_size_request(110, -1);
        capture.set_tooltip_text(Some("Press a key combination"));
        let capture_focus = capture.downgrade();
        capture.connect_clicked(move |_| {
            if let Some(button) = capture_focus.upgrade() {
                button.set_label("Press keys...");
                button.grab_focus();
            }
        });
        let key_controller = gtk4::EventControllerKey::new();
        let key_shortcuts = shortcuts.clone();
        let key_button = capture.downgrade();
        let key_status = shortcut_status.downgrade();
        key_controller.connect_key_pressed(move |_, key, _, modifiers| {
            let Some(button) = key_button.upgrade() else {
                return gtk4::glib::Propagation::Proceed;
            };
            let existing = key_shortcuts.display_label(action);
            if key == gdk::Key::Escape {
                button.set_label(&existing);
                return gtk4::glib::Propagation::Stop;
            }
            if matches!(
                key,
                gdk::Key::Alt_L
                    | gdk::Key::Alt_R
                    | gdk::Key::Control_L
                    | gdk::Key::Control_R
                    | gdk::Key::Shift_L
                    | gdk::Key::Shift_R
                    | gdk::Key::Super_L
                    | gdk::Key::Super_R
            ) {
                return gtk4::glib::Propagation::Stop;
            }
            let modifiers = modifiers
                & (gdk::ModifierType::CONTROL_MASK
                    | gdk::ModifierType::ALT_MASK
                    | gdk::ModifierType::SHIFT_MASK
                    | gdk::ModifierType::SUPER_MASK);
            let binding = Binding {
                key: key.to_lower(),
                modifiers,
            };
            if !gtk4::accelerator_valid(binding.key, binding.modifiers) {
                if let Some(status) = key_status.upgrade() {
                    status.set_label("Use a modifier with a letter or function key");
                }
                return gtk4::glib::Propagation::Stop;
            }
            match key_shortcuts.update(action, Some(binding.clone())) {
                Ok(()) => {
                    button.set_label(&binding.label());
                    if let Some(status) = key_status.upgrade() {
                        status.set_label(&key_shortcuts.status());
                    }
                }
                Err(error) => {
                    button.set_label(&existing);
                    if let Some(status) = key_status.upgrade() {
                        status.set_label(&error);
                    }
                }
            }
            gtk4::glib::Propagation::Stop
        });
        capture.add_controller(key_controller);
        row.append(&capture);

        let remove = gtk4::Button::from_icon_name("painto-close-symbolic");
        remove.set_tooltip_text(Some("Remove shortcut"));
        let remove_shortcuts = shortcuts.clone();
        let remove_button = capture.downgrade();
        let remove_status = shortcut_status.downgrade();
        remove.connect_clicked(move |_| {
            if let Err(error) = remove_shortcuts.update(action, None) {
                if let Some(status) = remove_status.upgrade() {
                    status.set_label(&error);
                }
            } else if let Some(button) = remove_button.upgrade() {
                button.set_label("Assign");
            }
        });
        row.append(&remove);
        shortcut_controls.append(&row);
    }
    if shortcuts.is_wayland() {
        let configure = gtk4::Button::with_label("Configure in desktop");
        let configure_shortcuts = shortcuts.clone();
        let configure_status = shortcut_status.downgrade();
        let configure_buttons = shortcut_buttons.clone();
        configure.connect_clicked(move |_| {
            let status = configure_status.clone();
            let buttons = configure_buttons.clone();
            let shortcuts = configure_shortcuts.clone();
            configure_shortcuts.configure_desktop(move |result| {
                if let Some(label) = status.upgrade() {
                    label.set_label(match result {
                        Ok(()) => "Desktop shortcut configuration opened",
                        Err(ref error) => error,
                    });
                }
                if result.is_ok() {
                    for (action, button) in buttons.borrow().iter() {
                        button.set_label(&shortcuts.display_label(*action));
                    }
                }
            });
        });
        shortcut_controls.append(&configure);
    }
    content.append(&shortcut_controls);
    content.append(&shortcut_status);
    if let Some(permission_button) = permission_button {
        let controls = shortcut_controls.downgrade();
        let button = permission_button.downgrade();
        let status = shortcut_status.downgrade();
        shortcuts.connect_permission_changed(move |permission| {
            let (Some(controls), Some(button), Some(status)) =
                (controls.upgrade(), button.upgrade(), status.upgrade())
            else {
                return;
            };
            let granted = permission.is_granted();
            controls.set_sensitive(granted);
            button.set_visible(!granted);
            button.set_sensitive(!matches!(permission, PermissionState::Requesting));
            status.set_label(match &permission {
                PermissionState::NotRequested => "Global shortcuts need desktop approval",
                PermissionState::Requesting => "Waiting for desktop approval...",
                PermissionState::Denied(error) => error,
                PermissionState::Granted => "",
            });
        });
    }
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
