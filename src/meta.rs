use gdk4::Display;
use gdk4::glib::GString;
use gdk4::prelude::*;
use gdk4_wayland::WaylandDisplay;
use gdk4_x11::X11Display;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Wayland,
    X11,
    Unknown,
}

pub struct Monitor {
    pub id: u32,
    pub connector: GString,
    pub geometry: (i32, i32, i32, i32), // (x, y, width, height)
    pub scale_factor: f64,
    pub refresh_rate: f64,
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

pub fn list_monitors(display: &Display) -> Vec<Monitor> {
    let monitors = display.monitors();
    (0..monitors.n_items())
        .filter_map(|i| monitors.item(i))
        .filter_map(|monitor| monitor.downcast::<gdk4::Monitor>().ok())
        .enumerate()
        .map(|(id, monitor)| {
            let connector = monitor.connector().unwrap_or_else(|| monitor_fallback(id));
            let geometry = monitor.geometry();

            Monitor {
                id: id as u32,
                connector,
                geometry: (
                    geometry.x(),
                    geometry.y(),
                    geometry.width(),
                    geometry.height(),
                ),
                scale_factor: monitor.scale_factor() as f64,
                refresh_rate: monitor.refresh_rate() as f64 / 1000.0,
            }
        })
        .collect()
}

pub fn monitor_fallback(index: usize) -> GString {
    GString::from(format!("Display {}", index + 1))
}
