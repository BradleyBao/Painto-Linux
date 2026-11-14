use gtk4::{gio, glib, prelude::*};
use painto_linux::meta::{Backend, check_backend};
use painto_linux::window::{drawing, toolbar};

fn main() -> glib::ExitCode {
    gio::resources_register_include!("icons.gresource").expect("Could not load bundled icons");
    let app = gtk4::Application::new(
        Some("com.tianyibrad.Painto"),
        gio::ApplicationFlags::FLAGS_NONE,
    );

    app.connect_activate(|app| {
        let display = gtk4::gdk::Display::default().expect("Could not connect to a display");
        gtk4::IconTheme::for_display(&display).add_resource_path("/com/tianyibrad/Painto/icons");
        let backend = check_backend();
        if backend == Backend::Unknown {
            eprintln!("Unsupported display backend");
            app.quit();
            return;
        }
        if backend == Backend::Wayland && !gtk4_layer_shell::is_supported() {
            eprintln!("This Wayland compositor does not support layer-shell");
            app.quit();
            return;
        }

        let (drawing_window, drawing_controller) = drawing::build_drawing_window(app, backend);

        if let Err(error) = drawing::set_click_through(&drawing_window, true, backend) {
            eprintln!("Could not change drawing window input region: {error}");
        }
        let toolbar_window =
            toolbar::build_toolbar_window(app, &drawing_window, &drawing_controller, backend);
        if let Err(error) = drawing::set_always_on_top(&toolbar_window, true, backend) {
            eprintln!("Could not keep toolbar above other windows: {error}");
        }
        if let Err(error) = drawing::set_always_on_top(&drawing_window, true, backend) {
            eprintln!("Could not keep drawing window above others: {error}");
        }
    });
    app.run()
}

#[cfg(test)]
mod tests {
    #[test]
    fn bundled_toolbar_icons_are_available() {
        gtk4::gio::resources_register_include!("icons.gresource").unwrap();
        for name in [
            "drag",
            "pointer",
            "pen",
            "eraser",
            "pixel-eraser",
            "clear",
            "close",
        ] {
            let path =
                format!("/com/tianyibrad/Painto/icons/scalable/actions/painto-{name}-symbolic.svg");
            let data =
                gtk4::gio::resources_lookup_data(&path, gtk4::gio::ResourceLookupFlags::NONE)
                    .unwrap_or_else(|error| panic!("Missing {path}: {error}"));
            assert!(!data.is_empty(), "Empty icon: {path}");
        }
    }
}
