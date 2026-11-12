use gtk4::{ApplicationWindow, prelude::*};

use super::drawing;

pub fn build_toolbar_window(
    app: &gtk4::Application,
    drawing_window: &ApplicationWindow,
) -> ApplicationWindow {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Painto Toolbar")
        .transient_for(drawing_window)
        .decorated(false)
        .resizable(false)
        .build();

    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    row.set_margin_start(8);
    row.set_margin_end(8);
    row.set_margin_top(6);
    row.set_margin_bottom(6);

    let drag_handle = gtk4::WindowHandle::new();
    let drag_label = gtk4::Label::new(Some("Move"));
    drag_label.set_margin_start(8);
    drag_label.set_margin_end(8);
    drag_handle.set_child(Some(&drag_label));
    drag_handle.set_tooltip_text(Some("Drag toolbar"));
    row.append(&drag_handle);
    row.append(&gtk4::Separator::new(gtk4::Orientation::Vertical));

    let computer = gtk4::ToggleButton::with_label("Computer");
    computer.set_tooltip_text(Some("Pass clicks through the drawing window"));
    computer.set_active(true);
    let pen = gtk4::ToggleButton::with_label("Draw");
    pen.set_tooltip_text(Some("Receive clicks on the drawing window"));
    pen.set_group(Some(&computer));

    for (button, click_through) in [(&computer, true), (&pen, false)] {
        let drawing_window = drawing_window.clone();
        button.connect_toggled(move |button| {
            if button.is_active()
                && let Err(error) = drawing::set_click_through(&drawing_window, click_through)
            {
                eprintln!("Could not change drawing window input region: {error}");
            }
        });
        row.append(button);
    }

    window.set_child(Some(&row));
    window.present();
    window
}
