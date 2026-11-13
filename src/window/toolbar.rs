use super::{drawing, ink::Tool};
use gtk4::{ApplicationWindow, prelude::*};

pub fn build_toolbar_window(
    app: &gtk4::Application,
    drawing_window: &ApplicationWindow,
    drawing_controller: &drawing::DrawingController,
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
    let object_eraser = gtk4::ToggleButton::with_label("Erase");
    object_eraser.set_tooltip_text(Some("Remove a whole stroke"));
    object_eraser.set_group(Some(&computer));
    let pixel_eraser = gtk4::ToggleButton::with_label("Pixel erase");
    pixel_eraser.set_tooltip_text(Some("Erase part of a stroke"));
    pixel_eraser.set_group(Some(&computer));

    for (button, tool) in [
        (&computer, Tool::Computer),
        (&pen, Tool::Pen),
        (&object_eraser, Tool::ObjectEraser),
        (&pixel_eraser, Tool::PixelEraser),
    ] {
        let drawing_window = drawing_window.clone();
        let controller = drawing_controller.clone();
        button.connect_toggled(move |button| {
            if button.is_active() {
                if let Err(error) =
                    drawing::set_click_through(&drawing_window, tool == Tool::Computer)
                {
                    eprintln!("Could not change drawing window input region: {error}");
                } else {
                    controller.set_tool(tool);
                }
            }
        });
        row.append(button);
    }

    row.append(&gtk4::Separator::new(gtk4::Orientation::Vertical));
    let color = gtk4::ColorButton::with_rgba(&gtk4::gdk::RGBA::BLACK);
    color.set_tooltip_text(Some("Pen color"));
    let color_controller = drawing_controller.clone();
    color.connect_color_set(move |button| color_controller.set_color(button.rgba()));
    row.append(&color);

    let width = gtk4::SpinButton::with_range(1.0, 40.0, 1.0);
    width.set_value(5.0);
    width.set_width_chars(2);
    width.set_tooltip_text(Some("Pen width"));
    let width_controller = drawing_controller.clone();
    width.connect_value_changed(move |button| width_controller.set_pen_width(button.value()));
    row.append(&width);

    // Clear Canva Button
    row.append(&gtk4::Separator::new(gtk4::Orientation::Vertical));
    let clear = gtk4::Button::from_icon_name("edit-clear-all-symbolic");
    clear.set_tooltip_text(Some("Clear all ink"));
    let clear_controller = drawing_controller.clone();
    clear.connect_clicked(move |_| clear_controller.clear());
    row.append(&clear);

    // Close Application Button
    let close = gtk4::Button::from_icon_name("window-close-symbolic");
    close.set_tooltip_text(Some("Close Painto"));
    let app_weak = app.downgrade();
    close.connect_clicked(move |_| {
        if let Some(app) = app_weak.upgrade() {
            app.quit();
        }
    });
    row.append(&close);

    window.set_child(Some(&row));
    window.present();
    window
}
