use gdk4_x11::X11Surface;
use gtk4::{ApplicationWindow, prelude::*};
use x11rb::{
    connection::Connection,
    protocol::{
        shape::{ConnectionExt as _, SK, SO},
        xproto::{ClientMessageEvent, ConnectionExt as _, EventMask},
    },
};

pub fn place_toolbar_bottom_left(
    window: &ApplicationWindow,
    monitor: &gtk4::gdk::Monitor,
) -> Result<(), String> {
    let surface = window.surface().ok_or("Window has no surface yet")?;
    let x11_surface = surface
        .clone()
        .downcast::<X11Surface>()
        .map_err(|_| "Window is not using X11")?;
    let bounds = monitor.geometry();
    let (connection, _) = x11rb::connect(None).map_err(|error| error.to_string())?;
    let window_id = x11_surface
        .xid()
        .try_into()
        .map_err(|_| "X11 window ID exceeds u32")?;
    let geometry = connection
        .get_geometry(window_id)
        .map_err(|error| error.to_string())?
        .reply()
        .map_err(|error| error.to_string())?;
    let scale = surface.scale_factor();
    let x = (bounds.x() + 16) * scale;
    let y = (bounds.y() + bounds.height() - 16) * scale - i32::from(geometry.height);
    connection
        .configure_window(
            window_id,
            &x11rb::protocol::xproto::ConfigureWindowAux::new().x(x).y(y),
        )
        .map_err(|error| error.to_string())?
        .check()
        .map_err(|error| error.to_string())?;
    connection.flush().map_err(|error| error.to_string())
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
