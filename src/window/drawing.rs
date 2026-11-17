use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use gtk4::{ApplicationWindow, CssProvider, cairo, prelude::*};

use crate::meta::Backend;

use super::{
    drawing_wayland, drawing_x11,
    ink::{Ink, Tool},
};

#[derive(Clone)]
pub struct DrawingController {
    ink: Rc<RefCell<Ink>>,
    canvases: Rc<RefCell<Vec<gtk4::DrawingArea>>>,
}

impl DrawingController {
    pub fn new() -> Self {
        Self {
            ink: Rc::new(RefCell::new(Ink::default())),
            canvases: Rc::new(RefCell::new(Vec::new())),
        }
    }

    fn queue_draw(&self) {
        Self::queue_canvases(&self.canvases);
    }

    fn queue_canvases(canvases: &Rc<RefCell<Vec<gtk4::DrawingArea>>>) {
        for canvas in canvases.borrow().iter() {
            canvas.queue_draw();
        }
    }

    pub fn set_canvases(&self, canvases: Vec<gtk4::DrawingArea>) {
        self.ink.borrow_mut().finish();
        *self.canvases.borrow_mut() = canvases;
        self.queue_draw();
    }

    pub fn set_tool(&self, tool: Tool) {
        self.ink.borrow_mut().set_tool(tool);
        self.queue_draw();
    }

    pub fn set_color(&self, color: gtk4::gdk::RGBA) {
        self.ink.borrow_mut().set_color(color);
    }

    pub fn set_pen_width(&self, width: f64) {
        self.ink.borrow_mut().set_pen_width(width);
    }

    pub fn clear(&self) {
        self.ink.borrow_mut().clear();
        self.queue_draw();
    }
}

impl Default for DrawingController {
    fn default() -> Self {
        Self::new()
    }
}

pub fn build_drawing_window(
    app: &gtk4::Application,
    backend: Backend,
    monitor: &gtk4::gdk::Monitor,
    controller: &DrawingController,
    active: Rc<Cell<bool>>,
) -> (ApplicationWindow, gtk4::DrawingArea) {
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
        drawing_wayland::prepare_drawing_window(&window, monitor);
    }

    let canvas = gtk4::DrawingArea::new();
    let (origin_x, origin_y) = (monitor.geometry().x() as f64, monitor.geometry().y() as f64);
    let draw_ink = controller.ink.clone();
    let draw_active = active;
    canvas.set_draw_func(move |_, context: &cairo::Context, _, _| {
        if !draw_active.get() {
            return;
        }
        context.translate(-origin_x, -origin_y);
        if let Err(error) = draw_ink.borrow().draw(context) {
            eprintln!("Could not draw ink: {error}");
        }
    });

    let drag = gtk4::GestureDrag::new();
    drag.set_button(1);
    let begin_ink = controller.ink.clone();
    let begin_canvases = Rc::downgrade(&controller.canvases);
    drag.connect_drag_begin(move |_, x, y| {
        begin_ink.borrow_mut().begin(x + origin_x, y + origin_y);
        if let Some(canvases) = begin_canvases.upgrade() {
            DrawingController::queue_canvases(&canvases);
        }
    });
    let move_ink = controller.ink.clone();
    let move_canvases = Rc::downgrade(&controller.canvases);
    drag.connect_drag_update(move |gesture, dx, dy| {
        if let Some((start_x, start_y)) = gesture.start_point() {
            move_ink
                .borrow_mut()
                .move_to(start_x + dx + origin_x, start_y + dy + origin_y);
            if let Some(canvases) = move_canvases.upgrade() {
                DrawingController::queue_canvases(&canvases);
            }
        }
    });
    let end_ink = controller.ink.clone();
    let end_canvases = Rc::downgrade(&controller.canvases);
    drag.connect_drag_end(move |gesture, dx, dy| {
        if let Some((start_x, start_y)) = gesture.start_point() {
            end_ink
                .borrow_mut()
                .move_to(start_x + dx + origin_x, start_y + dy + origin_y);
        }
        end_ink.borrow_mut().finish();
        if let Some(canvases) = end_canvases.upgrade() {
            DrawingController::queue_canvases(&canvases);
        }
    });
    let cancel_ink = controller.ink.clone();
    let cancel_canvases = Rc::downgrade(&controller.canvases);
    drag.connect_cancel(move |_, _| {
        cancel_ink.borrow_mut().finish();
        if let Some(canvases) = cancel_canvases.upgrade() {
            DrawingController::queue_canvases(&canvases);
        }
    });
    canvas.add_controller(drag);

    window.set_child(Some(&canvas));
    if backend == Backend::X11 {
        window.fullscreen_on_monitor(monitor);
    }
    window.present();
    (window, canvas)
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
