use gtk4::{gio, glib, prelude::*};
use painto_linux::meta::{Backend, check_backend};
use painto_linux::window::{drawing, toolbar};

fn main() -> glib::ExitCode {
    let app = gtk4::Application::new(
        Some("com.tianyibrad.Painto"),
        gio::ApplicationFlags::FLAGS_NONE,
    );

    app.connect_activate(|app| {
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
