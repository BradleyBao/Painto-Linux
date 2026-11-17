use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use crate::meta::{Backend, monitor_fallback};
use gtk4::{ApplicationWindow, gdk, prelude::*};
use gtk4_layer_shell::LayerShell;

use super::{display_settings::DisplaySettings, drawing, drawing_x11};

#[derive(Clone)]
pub struct OverlayManager(Rc<RefCell<OverlayState>>);

struct OverlayState {
    app: gtk4::Application,
    backend: Backend,
    controller: drawing::DrawingController,
    settings: DisplaySettings,
    windows: Vec<CanvasWindow>,
    toolbar: Option<gtk4::glib::WeakRef<ApplicationWindow>>,
    click_through: bool,
}

#[derive(Clone)]
struct CanvasWindow {
    connector: String,
    monitor: gdk::Monitor,
    window: ApplicationWindow,
    canvas: gtk4::DrawingArea,
    active: Rc<Cell<bool>>,
}

impl OverlayManager {
    pub fn new(app: &gtk4::Application, backend: Backend) -> Self {
        Self(Rc::new(RefCell::new(OverlayState {
            app: app.clone(),
            backend,
            controller: drawing::DrawingController::new(),
            settings: DisplaySettings::load(),
            windows: Vec::new(),
            toolbar: None,
            click_through: true,
        })))
    }

    pub fn controller(&self) -> drawing::DrawingController {
        self.0.borrow().controller.clone()
    }

    pub fn settings(&self) -> DisplaySettings {
        self.0.borrow().settings.clone()
    }

    pub fn primary_window(&self) -> Option<ApplicationWindow> {
        let state = self.0.borrow();
        state
            .windows
            .iter()
            .find(|entry| entry.active.get())
            .or_else(|| state.windows.first())
            .map(|entry| entry.window.clone())
    }

    pub fn set_toolbar(&self, toolbar: &ApplicationWindow) {
        self.0.borrow_mut().toolbar = Some(toolbar.downgrade());
    }

    pub fn selected_monitor(&self) -> Option<gdk::Monitor> {
        let state = self.0.borrow();
        state
            .windows
            .iter()
            .find(|entry| entry.connector == state.settings.connector)
            .or_else(|| state.windows.first())
            .map(|entry| entry.monitor.clone())
    }

    pub fn set_click_through(&self, enabled: bool) -> Result<(), String> {
        let state = self.0.borrow();
        for entry in &state.windows {
            drawing::set_click_through(
                &entry.window,
                enabled || !entry.active.get(),
                state.backend,
            )?;
        }
        drop(state);
        self.0.borrow_mut().click_through = enabled;
        Ok(())
    }

    pub fn update_settings(&self, settings: DisplaySettings) -> Result<(), String> {
        let previous = self.settings();
        if !(settings.all_monitors && previous.all_monitors) {
            self.apply(&settings)?;
        }
        self.0.borrow_mut().settings = settings;
        self.0.borrow().settings.save()?;
        Ok(())
    }

    pub fn apply_saved(&self) -> Result<(), String> {
        let settings = self.settings();
        self.apply(&settings)
    }

    fn apply(&self, settings: &DisplaySettings) -> Result<(), String> {
        let state = self.0.borrow();
        let display = gdk::Display::default().ok_or("No display available")?;
        let model = display.monitors();
        let monitors: Vec<gdk::Monitor> = (0..model.n_items())
            .filter_map(|index| model.item(index))
            .filter_map(|item| item.downcast::<gdk::Monitor>().ok())
            .collect();
        if monitors.is_empty() {
            return Err("No monitors available".into());
        }
        let selected = monitors
            .iter()
            .enumerate()
            .find(|(index, monitor)| {
                monitor
                    .connector()
                    .unwrap_or_else(|| monitor_fallback(*index))
                    .as_str()
                    == settings.connector
            })
            .map(|(_, monitor)| monitor)
            .unwrap_or(&monitors[0]);
        let targets: Vec<(usize, &gdk::Monitor)> =
            if state.backend == Backend::Wayland || settings.all_monitors {
                monitors.iter().enumerate().collect()
            } else {
                vec![(
                    monitors
                        .iter()
                        .position(|monitor| monitor == selected)
                        .unwrap(),
                    selected,
                )]
            };
        let mut windows = Vec::new();
        let mut created: Vec<ApplicationWindow> = Vec::new();
        for (index, monitor) in targets {
            let connector = monitor
                .connector()
                .unwrap_or_else(|| monitor_fallback(index))
                .to_string();
            let is_active = settings.all_monitors || monitor == selected;
            if let Some(existing) = state
                .windows
                .iter()
                .find(|entry| entry.connector == connector)
            {
                if existing.active.get() != is_active {
                    existing.active.set(is_active);
                    existing.canvas.queue_draw();
                    drawing::set_click_through(
                        &existing.window,
                        state.click_through || !is_active,
                        state.backend,
                    )?;
                }
                windows.push(existing.clone());
                continue;
            }
            let active = Rc::new(Cell::new(is_active));
            let (window, canvas) = drawing::build_drawing_window(
                &state.app,
                state.backend,
                monitor,
                &state.controller,
                active.clone(),
            );
            if let Err(error) = drawing::set_click_through(
                &window,
                state.click_through || !is_active,
                state.backend,
            )
            .and_then(|_| drawing::set_always_on_top(&window, true, state.backend))
            {
                window.close();
                for new_window in created {
                    new_window.close();
                }
                return Err(error);
            }
            created.push(window.clone());
            windows.push(CanvasWindow {
                connector,
                monitor: monitor.clone(),
                window,
                canvas,
                active,
            });
        }
        let backend = state.backend;
        let toolbar = state.toolbar.as_ref().and_then(|toolbar| toolbar.upgrade());
        let reposition_toolbar = state.backend == Backend::Wayland
            && !settings.all_monitors
            && state.settings.connector != settings.connector;
        if let Some(toolbar) = &toolbar
            && state.backend == Backend::X11
        {
            toolbar.set_transient_for(windows.first().map(|entry| &entry.window));
        }
        drop(state);
        let mut state = self.0.borrow_mut();
        state
            .controller
            .set_canvases(windows.iter().map(|entry| entry.canvas.clone()).collect());
        for old in std::mem::replace(&mut state.windows, windows) {
            if !state.windows.iter().any(|entry| entry.window == old.window) {
                old.window.close();
            }
        }
        drop(state);
        if reposition_toolbar && let Some(toolbar) = toolbar {
            let monitor = selected.clone();
            gtk4::glib::idle_add_local_once(move || {
                toolbar.set_monitor(Some(&monitor));
            });
        } else if let Some(toolbar) = toolbar
            && backend == Backend::X11
        {
            let monitor = selected.clone();
            gtk4::glib::idle_add_local_once(move || {
                if let Err(error) = drawing_x11::place_toolbar_bottom_left(&toolbar, &monitor) {
                    eprintln!("Could not place toolbar at bottom-left: {error}");
                }
            });
        }
        Ok(())
    }
}
