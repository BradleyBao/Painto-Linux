use std::{cell::RefCell, rc::Rc};

use gtk4::{ApplicationWindow, CssProvider, cairo, prelude::*};

use crate::meta::Backend;

use super::{
    drawing_wayland, drawing_x11,
    ink::{Ink, Tool},
};

#[derive(Clone)]
pub struct DrawingController {
    ink: Rc<RefCell<Ink>>,
    canvas: gtk4::DrawingArea,
}

impl DrawingController {
    pub fn set_tool(&self, tool: Tool) {
        self.ink.borrow_mut().set_tool(tool);
        self.canvas.queue_draw();
    }

    pub fn set_color(&self, color: gtk4::gdk::RGBA) {
        self.ink.borrow_mut().set_color(color);
    }

    pub fn set_pen_width(&self, width: f64) {
        self.ink.borrow_mut().set_pen_width(width);
    }

    pub fn clear(&self) {
        self.ink.borrow_mut().clear();
        self.canvas.queue_draw();
    }
}

pub fn build_drawing_window(
    app: &gtk4::Application,
    backend: Backend,
) -> (ApplicationWindow, DrawingController) {
    let provider = CssProvider::new();
    provider.load_from_data("window.painto-canvas { background-color: transparent; }");
    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().expect("Could not connect to a display"),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Painto")
        .default_width(800)
        .default_height(600)
        .build();
    window.add_css_class("painto-canvas");
    if backend == Backend::Wayland {
        drawing_wayland::prepare_drawing_window(&window);
    }

    let canvas = gtk4::DrawingArea::new();
    let ink = Rc::new(RefCell::new(Ink::default()));
    let draw_ink = ink.clone();
    canvas.set_draw_func(move |_, context: &cairo::Context, _, _| {
        if let Err(error) = draw_ink.borrow().draw(context) {
            eprintln!("Could not draw ink: {error}");
        }
    });

    let drag = gtk4::GestureDrag::new();
    drag.set_button(1);
    let begin_ink = ink.clone();
    let begin_canvas = canvas.clone();
    drag.connect_drag_begin(move |_, x, y| {
        begin_ink.borrow_mut().begin(x, y);
        begin_canvas.queue_draw();
    });
    let move_ink = ink.clone();
    let move_canvas = canvas.clone();
    drag.connect_drag_update(move |gesture, dx, dy| {
        if let Some((start_x, start_y)) = gesture.start_point() {
            move_ink.borrow_mut().move_to(start_x + dx, start_y + dy);
            move_canvas.queue_draw();
        }
    });
    let end_ink = ink.clone();
    let end_canvas = canvas.clone();
    drag.connect_drag_end(move |gesture, dx, dy| {
        if let Some((start_x, start_y)) = gesture.start_point() {
            end_ink.borrow_mut().move_to(start_x + dx, start_y + dy);
        }
        end_ink.borrow_mut().finish();
        end_canvas.queue_draw();
    });
    let cancel_ink = ink.clone();
    let cancel_canvas = canvas.clone();
    drag.connect_cancel(move |_, _| {
        cancel_ink.borrow_mut().finish();
        cancel_canvas.queue_draw();
    });
    canvas.add_controller(drag);

    window.set_child(Some(&canvas));
    if backend == Backend::X11 {
        window.fullscreen();
    }
    window.present();
    (window, DrawingController { ink, canvas })
}

pub fn set_click_through(
    window: &ApplicationWindow,
    click_through: bool,
    backend: Backend,
) -> Result<(), String> {
    match backend {
        Backend::X11 => drawing_x11::set_click_through(window, click_through),
        Backend::Wayland => drawing_wayland::set_click_through(window, click_through),
        Backend::Unknown => Err("Unsupported display backend".into()),
    }
}

pub fn set_always_on_top(
    window: &ApplicationWindow,
    always_on_top: bool,
    backend: Backend,
) -> Result<(), String> {
    match backend {
        Backend::X11 => drawing_x11::set_always_on_top(window, always_on_top),
        Backend::Wayland => drawing_wayland::set_always_on_top(window, always_on_top),
        Backend::Unknown => Err("Unsupported display backend".into()),
    }
}
