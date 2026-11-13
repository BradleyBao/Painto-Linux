use std::{cell::RefCell, rc::Rc};

use gdk4_x11::X11Surface;
use gtk4::{ApplicationWindow, CssProvider, cairo, prelude::*};
use x11rb::{
    connection::Connection,
    protocol::{
        shape::{ConnectionExt as _, SK, SO},
        xproto::{ClientMessageEvent, ConnectionExt as _, EventMask},
    },
};

use super::ink::{Ink, Tool};

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

pub fn build_drawing_window(app: &gtk4::Application) -> (ApplicationWindow, DrawingController) {
    let provider = CssProvider::new();
    provider.load_from_data("window { background-color: transparent; }");
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
    window.fullscreen();
    window.present();
    (window, DrawingController { ink, canvas })
}

pub fn set_click_through(window: &ApplicationWindow, click_through: bool) -> Result<(), String> {
    let surface = window.surface().ok_or("Window has no surface yet")?;
    let x11_surface = surface
        .downcast::<X11Surface>()
        .map_err(|_| "Window is not using X11")?;
    let window_id = x11_surface
        .xid()
        .try_into()
        .map_err(|_| "X11 window ID exceeds u32")?;
    let (connection, _) = x11rb::connect(None).map_err(|error| error.to_string())?;
    if click_through {
        connection
            .shape_rectangles(
                SO::SET,
                SK::INPUT,
                x11rb::protocol::xproto::ClipOrdering::UNSORTED,
                window_id,
                0,
                0,
                &[],
            )
            .map_err(|error| error.to_string())?
            .check()
            .map_err(|error| error.to_string())?;
    } else {
        // A source bitmap of None removes the custom shape and follows future resizes.
        connection
            .shape_mask(SO::SET, SK::INPUT, window_id, 0, 0, 0u32)
            .map_err(|error| error.to_string())?
            .check()
            .map_err(|error| error.to_string())?;
    }
    connection.flush().map_err(|error| error.to_string())
}

pub fn set_always_on_top(window: &ApplicationWindow, always_on_top: bool) -> Result<(), String> {
    let surface = window.surface().ok_or("Window has no surface yet")?;
    let x11_surface = surface
        .downcast::<X11Surface>()
        .map_err(|_| "Window is not using X11")?;
    let (connection, screen_number) = x11rb::connect(None).map_err(|error| error.to_string())?;
    let root = connection.setup().roots[screen_number].root;
    let state = connection
        .intern_atom(false, b"_NET_WM_STATE")
        .map_err(|error| error.to_string())?
        .reply()
        .map_err(|error| error.to_string())?
        .atom;
    let above = connection
        .intern_atom(false, b"_NET_WM_STATE_ABOVE")
        .map_err(|error| error.to_string())?
        .reply()
        .map_err(|error| error.to_string())?
        .atom;
    // EWMH asks the window manager to add or remove the ABOVE state.
    let window_id = x11_surface
        .xid()
        .try_into()
        .map_err(|_| "X11 window ID exceeds u32")?;
    let event = ClientMessageEvent::new(
        32,
        window_id,
        state,
        [if always_on_top { 1 } else { 0 }, above, 0, 1, 0],
    );
    connection
        .send_event(
            false,
            root,
            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
            event,
        )
        .map_err(|error| error.to_string())?
        .check()
        .map_err(|error| error.to_string())?;
    connection.flush().map_err(|error| error.to_string())
}
