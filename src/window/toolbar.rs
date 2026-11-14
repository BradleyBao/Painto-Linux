use super::{drawing, ink::Tool};
use crate::meta::Backend;
use gtk4::{ApplicationWindow, prelude::*};

use super::{drawing_wayland, drawing_x11};

pub fn build_toolbar_window(
    app: &gtk4::Application,
    drawing_window: &ApplicationWindow,
    drawing_controller: &drawing::DrawingController,
    backend: Backend,
) -> ApplicationWindow {
    let builder = ApplicationWindow::builder()
        .application(app)
        .title("Painto Toolbar")
        .decorated(false)
        .resizable(false);
    let window = if backend == Backend::Wayland {
        let window = builder.build();
        drawing_wayland::prepare_toolbar_window(&window);
        window
    } else {
        builder.transient_for(drawing_window).build()
    };

    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    row.set_margin_start(8);
    row.set_margin_end(8);
    row.set_margin_top(6);
    row.set_margin_bottom(6);

    let drag_icon = gtk4::Image::from_icon_name("painto-drag-symbolic");
    drag_icon.set_margin_start(12);
    drag_icon.set_margin_end(12);
    drag_icon.set_margin_top(12);
    drag_icon.set_margin_bottom(12);
    let drag_area = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    drag_area.set_size_request(40, 40);
    drag_area.set_halign(gtk4::Align::Center);
    drag_area.set_valign(gtk4::Align::Center);
    drag_area.set_tooltip_text(Some("Drag toolbar"));
    drag_area.append(&drag_icon);
    if backend == Backend::Wayland {
        drag_area.set_cursor_from_name(Some("grab"));
        let drag = gtk4::GestureDrag::new();
        drag.set_button(1);
        let drag_window = window.clone();
        drag.connect_drag_update(move |_, dx, dy| {
            drawing_wayland::move_toolbar_by(&drag_window, dx, dy);
        });
        drag_area.add_controller(drag);
        row.append(&drag_area);
    } else {
        let drag_handle = gtk4::WindowHandle::new();
        drag_handle.set_child(Some(&drag_area));
        row.append(&drag_handle);
    }
    row.append(&gtk4::Separator::new(gtk4::Orientation::Vertical));

    let computer = gtk4::ToggleButton::new();
    computer.set_label("Computer Mode");
    computer.set_icon_name("painto-pointer-symbolic");
    computer.set_tooltip_text(Some("Pass clicks through the drawing window"));
    computer.set_active(true);
    let pen = gtk4::ToggleButton::new();
    pen.set_label("Draw");
    pen.set_icon_name("painto-pen-symbolic");
    pen.set_tooltip_text(Some("Receive clicks on the drawing window"));
    pen.set_group(Some(&computer));
    let object_eraser = gtk4::ToggleButton::new();
    object_eraser.set_label("Erase");
    object_eraser.set_icon_name("painto-eraser-symbolic");
    object_eraser.set_tooltip_text(Some("Remove a whole stroke"));
    object_eraser.set_group(Some(&computer));
    let pixel_eraser = gtk4::ToggleButton::new();
    pixel_eraser.set_label("Pixel erase");
    pixel_eraser.set_icon_name("painto-pixel-eraser-symbolic");
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
                    drawing::set_click_through(&drawing_window, tool == Tool::Computer, backend)
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
    // A regular color dialog appears below layer-shell surfaces on Wayland.
    if backend == Backend::Wayland {
        let color = gtk4::MenuButton::new();
        color.set_direction(gtk4::ArrowType::Up);
        color.set_tooltip_text(Some("Pen color"));
        let swatch = gtk4::DrawingArea::new();
        swatch.set_content_width(22);
        swatch.set_content_height(22);
        let chooser = gtk4::ColorChooserWidget::new();
        chooser.set_use_alpha(false);
        chooser.set_rgba(&gtk4::gdk::RGBA::BLACK);
        let draw_chooser = chooser.downgrade();
        swatch.set_draw_func(move |_, context, width, height| {
            if let Some(chooser) = draw_chooser.upgrade() {
                let rgba = chooser.rgba();
                context.set_source_rgba(
                    rgba.red() as f64,
                    rgba.green() as f64,
                    rgba.blue() as f64,
                    1.0,
                );
                context.rectangle(0.0, 0.0, width as f64, height as f64);
                let _ = context.fill();
            }
        });
        color.set_child(Some(&swatch));
        let popover = gtk4::Popover::new();
        popover.set_child(Some(&chooser));
        color.set_popover(Some(&popover));
        let color_controller = drawing_controller.clone();
        chooser.connect_rgba_notify(move |chooser| {
            color_controller.set_color(chooser.rgba());
            swatch.queue_draw();
        });
        let close_popover = popover.downgrade();
        chooser.connect_color_activated(move |_, _| {
            if let Some(popover) = close_popover.upgrade() {
                popover.popdown();
            }
        });
        row.append(&color);
    } else {
        let color = gtk4::ColorButton::with_rgba(&gtk4::gdk::RGBA::BLACK);
        color.set_tooltip_text(Some("Pen color"));
        let color_controller = drawing_controller.clone();
        color.connect_color_set(move |button| color_controller.set_color(button.rgba()));
        row.append(&color);
    }

    let width = gtk4::SpinButton::with_range(1.0, 40.0, 1.0);
    width.set_value(5.0);
    width.set_width_chars(2);
    width.set_tooltip_text(Some("Pen width"));
    let width_controller = drawing_controller.clone();
    width.connect_value_changed(move |button| width_controller.set_pen_width(button.value()));
    row.append(&width);

    // Clear Canva Button
    row.append(&gtk4::Separator::new(gtk4::Orientation::Vertical));
    let clear = gtk4::Button::from_icon_name("painto-clear-symbolic");
    clear.set_tooltip_text(Some("Clear all ink"));
    let clear_controller = drawing_controller.clone();
    clear.connect_clicked(move |_| clear_controller.clear());
    row.append(&clear);

    // Close Application Button
    let close = gtk4::Button::from_icon_name("painto-close-symbolic");
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
    if backend == Backend::X11 {
        let placed_window = window.clone();
        gtk4::glib::idle_add_local_once(move || {
            if let Err(error) = drawing_x11::place_toolbar_bottom_left(&placed_window) {
                eprintln!("Could not place toolbar at bottom-left: {error}");
            }
        });
    }
    window
}
