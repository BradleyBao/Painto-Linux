use gdk4_x11::X11Surface;
use gtk4::{ApplicationWindow, CssProvider, cairo, prelude::*};
use x11rb::{
    connection::Connection,
    protocol::{
        shape::{ConnectionExt as _, SK, SO},
        xproto::{ClientMessageEvent, ConnectionExt as _, EventMask},
    },
};

pub fn build_drawing_window(app: &gtk4::Application) -> ApplicationWindow {
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
    canvas.set_draw_func(|_, context: &cairo::Context, _, _| {
        // Cairo drawing for the future annotation canvas goes here.
        context.set_operator(gtk4::cairo::Operator::Source);
        context.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        context.paint().expect("Invalid cairo context");
        context.set_operator(gtk4::cairo::Operator::Over); // Over for normal alpha blending
    });

    window.set_child(Some(&canvas));
    window.present();
    window
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
