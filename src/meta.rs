use gdk4::Display;
use gdk4::prelude::*;
use gdk4_wayland::WaylandDisplay;
use gdk4_x11::X11Display;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Wayland,
    X11,
    Unknown,
}

pub fn check_backend() -> Backend {
    if let Some(display) = Display::default() {
        if display.is::<WaylandDisplay>() {
            Backend::Wayland
        } else if display.is::<X11Display>() {
            Backend::X11
        } else {
            Backend::Unknown
        }
    } else {
        Backend::Unknown
    }
}
