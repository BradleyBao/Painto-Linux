use gdk4_wayland::WaylandSurface;
use gtk4::{ApplicationWindow, cairo, prelude::*};
use gtk4_layer_shell::{Edge, Layer, LayerShell};

pub fn prepare_drawing_window(window: &ApplicationWindow) {
    window.init_layer_shell();
    window.set_namespace(Some("painto-canvas"));
    window.set_layer(Layer::Top);
    window.set_exclusive_zone(-1);
    for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
        window.set_anchor(edge, true);
    }
}

pub fn prepare_toolbar_window(window: &ApplicationWindow) {
    window.init_layer_shell();
    window.set_namespace(Some("painto-toolbar"));
    window.set_layer(Layer::Overlay);
    window.set_exclusive_zone(-1);
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Left, true);
    window.set_margin(Edge::Top, 16);
    window.set_margin(Edge::Left, 16);
}

pub fn move_toolbar_by(window: &ApplicationWindow, dx: f64, dy: f64) {
    let Some(surface) = window.surface() else {
        return;
    };
    let Some(monitor) = surface.display().monitor_at_surface(&surface) else {
        return;
    };
    let bounds = monitor.geometry();
    let max_x = (bounds.width() - window.width()).max(0);
    let max_y = (bounds.height() - window.height()).max(0);
    let x = (window.margin(Edge::Left) + dx.round() as i32).clamp(0, max_x);
    let y = (window.margin(Edge::Top) + dy.round() as i32).clamp(0, max_y);
    window.set_margin(Edge::Left, x);
    window.set_margin(Edge::Top, y);
}

pub fn set_click_through(window: &ApplicationWindow, click_through: bool) -> Result<(), String> {
    let surface = window.surface().ok_or("Window has no surface yet")?;
    if !surface.is::<WaylandSurface>() {
        return Err("Window is not using Wayland".into());
    }
    if click_through {
        surface.set_input_region(Some(&cairo::Region::create()));
    } else {
        surface.set_input_region(None);
    }
    Ok(())
}

pub fn set_always_on_top(window: &ApplicationWindow, always_on_top: bool) -> Result<(), String> {
    if !window.is_layer_window() {
        return Err("Window is not a Wayland layer surface".into());
    }
    if always_on_top {
        if window.layer() != Layer::Overlay {
            window.set_layer(Layer::Top);
        }
    } else {
        window.set_layer(Layer::Bottom);
    }
    Ok(())
}
