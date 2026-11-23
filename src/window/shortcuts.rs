use std::{cell::RefCell, collections::HashMap, rc::Rc};

use ashpd::desktop::{
    CreateSessionOptions, Session,
    global_shortcuts::{
        BindShortcutsOptions, ConfigureShortcutsOptions, GlobalShortcuts, NewShortcut,
    },
};
use futures_util::{StreamExt, future::Either};
use glib::translate::IntoGlib;
use gtk4::{gdk, glib};
use x11rb::{
    connection::Connection,
    protocol::{
        Event,
        xproto::{ConnectionExt as _, GrabMode, ModMask},
    },
    rust_connection::RustConnection,
};

use crate::meta::Backend;

const CONFIG_GROUP: &str = "Shortcuts";
const PERMISSION_KEY: &str = "permission-granted";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PermissionState {
    NotRequested,
    Requesting,
    Granted,
    Denied(String),
}

impl PermissionState {
    pub fn is_granted(&self) -> bool {
        matches!(self, Self::Granted)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Action {
    Computer,
    Pen,
    Eraser,
    PixelEraser,
    Clear,
    Close,
}

impl Action {
    pub const ALL: [Self; 6] = [
        Self::Computer,
        Self::Pen,
        Self::Eraser,
        Self::PixelEraser,
        Self::Clear,
        Self::Close,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Computer => "computer",
            Self::Pen => "pen",
            Self::Eraser => "eraser",
            Self::PixelEraser => "pixel-eraser",
            Self::Clear => "clear",
            Self::Close => "close",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Computer => "Computer mode",
            Self::Pen => "Pen mode",
            Self::Eraser => "Erase stroke",
            Self::PixelEraser => "Pixel erase",
            Self::Clear => "Clear all",
            Self::Close => "Close Painto",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    pub key: gdk::Key,
    pub modifiers: gdk::ModifierType,
}

impl Binding {
    pub fn parse(value: &str) -> Option<Self> {
        let mut modifiers = gdk::ModifierType::empty();
        let mut pieces = value.split('+').peekable();
        while let Some(piece) = pieces.next() {
            if pieces.peek().is_none() {
                let key = gdk::Key::from_name(piece)?.to_lower();
                return Some(Self { key, modifiers });
            }
            let modifier = match piece {
                "Ctrl" => gdk::ModifierType::CONTROL_MASK,
                "Alt" => gdk::ModifierType::ALT_MASK,
                "Shift" => gdk::ModifierType::SHIFT_MASK,
                "Super" => gdk::ModifierType::SUPER_MASK,
                _ => return None,
            };
            if modifiers.contains(modifier) {
                return None;
            }
            modifiers |= modifier;
        }
        None
    }

    pub fn name(&self) -> String {
        let mut parts = Vec::new();
        if self.modifiers.contains(gdk::ModifierType::CONTROL_MASK) {
            parts.push("Ctrl");
        }
        if self.modifiers.contains(gdk::ModifierType::ALT_MASK) {
            parts.push("Alt");
        }
        if self.modifiers.contains(gdk::ModifierType::SHIFT_MASK) {
            parts.push("Shift");
        }
        if self.modifiers.contains(gdk::ModifierType::SUPER_MASK) {
            parts.push("Super");
        }
        let name = self.key.name().unwrap_or_default();
        parts.push(&name);
        parts.join("+")
    }

    pub fn label(&self) -> String {
        gtk4::accelerator_get_label(self.key, self.modifiers).to_string()
    }

    fn portal_trigger(&self) -> String {
        let mut parts = Vec::new();
        if self.modifiers.contains(gdk::ModifierType::CONTROL_MASK) {
            parts.push("CTRL");
        }
        if self.modifiers.contains(gdk::ModifierType::ALT_MASK) {
            parts.push("ALT");
        }
        if self.modifiers.contains(gdk::ModifierType::SHIFT_MASK) {
            parts.push("SHIFT");
        }
        if self.modifiers.contains(gdk::ModifierType::SUPER_MASK) {
            parts.push("LOGO");
        }
        let name = self.key.name();
        parts.push(name.as_deref().unwrap_or(""));
        parts.join("+")
    }
}

#[derive(Clone)]
pub struct ShortcutSettings(HashMap<Action, Binding>);

impl Default for ShortcutSettings {
    fn default() -> Self {
        let mut bindings = HashMap::new();
        for (action, key) in [
            (Action::Computer, "c"),
            (Action::Pen, "b"),
            (Action::Eraser, "e"),
        ] {
            bindings.insert(
                action,
                Binding {
                    key: gdk::Key::from_name(key).unwrap(),
                    modifiers: gdk::ModifierType::ALT_MASK,
                },
            );
        }
        Self(bindings)
    }
}

impl ShortcutSettings {
    pub fn load() -> Self {
        let path = glib::user_config_dir().join("painto/settings.ini");
        Self::load_from(&path)
    }

    fn load_from(path: &std::path::Path) -> Self {
        let file = glib::KeyFile::new();
        if file.load_from_file(path, glib::KeyFileFlags::NONE).is_err() {
            return Self::default();
        }
        let mut settings = Self::default();
        for action in Action::ALL {
            if let Ok(value) = file.string(CONFIG_GROUP, action.id()) {
                match Binding::parse(&value) {
                    Some(binding) => {
                        if let Err(error) = settings.set(action, Some(binding)) {
                            eprintln!("Ignoring shortcut for {}: {error}", action.id());
                        }
                    }
                    None if value.is_empty() => {
                        settings.set(action, None).unwrap();
                    }
                    None => eprintln!("Ignoring invalid shortcut for {}", action.id()),
                }
            }
        }
        settings
    }

    fn permission_was_granted() -> bool {
        let path = glib::user_config_dir().join("painto/settings.ini");
        Self::permission_was_granted_at(&path)
    }

    fn permission_was_granted_at(path: &std::path::Path) -> bool {
        let file = glib::KeyFile::new();
        file.load_from_file(path, glib::KeyFileFlags::NONE).is_ok()
            && file.boolean(CONFIG_GROUP, PERMISSION_KEY).unwrap_or(false)
    }

    fn save_permission_granted(granted: bool) -> Result<(), String> {
        let path = glib::user_config_dir().join("painto/settings.ini");
        Self::save_permission_granted_to(&path, granted)
    }

    fn save_permission_granted_to(path: &std::path::Path, granted: bool) -> Result<(), String> {
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|error| error.to_string())?;
        let file = glib::KeyFile::new();
        if path.exists() {
            file.load_from_file(path, glib::KeyFileFlags::NONE)
                .map_err(|error| error.to_string())?;
        }
        file.set_boolean(CONFIG_GROUP, PERMISSION_KEY, granted);
        file.save_to_file(path).map_err(|error| error.to_string())
    }

    pub fn get(&self, action: Action) -> Option<&Binding> {
        self.0.get(&action)
    }

    pub fn set(&mut self, action: Action, binding: Option<Binding>) -> Result<(), String> {
        if let Some(ref binding) = binding
            && self
                .0
                .iter()
                .any(|(other, existing)| *other != action && *existing == *binding)
        {
            return Err("Shortcut already assigned to another action".into());
        }
        if let Some(binding) = binding {
            self.0.insert(action, binding);
        } else {
            self.0.remove(&action);
        }
        Ok(())
    }

    pub fn save(&self) -> Result<(), String> {
        let path = glib::user_config_dir().join("painto/settings.ini");
        self.save_to(&path)
    }

    fn save_to(&self, path: &std::path::Path) -> Result<(), String> {
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|error| error.to_string())?;
        let file = glib::KeyFile::new();
        if path.exists() {
            file.load_from_file(path, glib::KeyFileFlags::NONE)
                .map_err(|error| error.to_string())?;
        }
        for action in Action::ALL {
            file.set_string(
                CONFIG_GROUP,
                action.id(),
                &self.get(action).map(Binding::name).unwrap_or_default(),
            );
        }
        file.save_to_file(path).map_err(|error| error.to_string())
    }
}

struct X11State {
    connection: RustConnection,
    root: u32,
    registered: HashMap<(u8, u16), Action>,
}

impl X11State {
    fn new() -> Result<Self, String> {
        let (connection, screen) = x11rb::connect(None).map_err(|error| error.to_string())?;
        let root = connection.setup().roots[screen].root;
        Ok(Self {
            connection,
            root,
            registered: HashMap::new(),
        })
    }

    fn keycode(&self, key: gdk::Key) -> Result<u8, String> {
        let setup = self.connection.setup();
        let first = setup.min_keycode;
        let count = setup.max_keycode - first + 1;
        let mapping = self
            .connection
            .get_keyboard_mapping(first, count)
            .map_err(|error| error.to_string())?
            .reply()
            .map_err(|error| error.to_string())?;
        let target: u32 = key.into_glib();
        mapping
            .keysyms
            .chunks(mapping.keysyms_per_keycode as usize)
            .position(|symbols| symbols.contains(&target))
            .map(|index| first + index as u8)
            .ok_or_else(|| {
                format!(
                    "Key {} is not available in this X11 layout",
                    key.name().unwrap_or_default()
                )
            })
    }

    fn register(&mut self, settings: &ShortcutSettings) -> Result<(), String> {
        self.clear();
        let result = self.register_inner(settings);
        if result.is_err() {
            self.clear();
        }
        result
    }

    fn register_inner(&mut self, settings: &ShortcutSettings) -> Result<(), String> {
        for action in Action::ALL {
            let Some(binding) = settings.get(action) else {
                continue;
            };
            let code = self.keycode(binding.key)?;
            let mask = modifier_mask(binding.modifiers);
            for extra in [
                ModMask::from(0u16),
                ModMask::LOCK,
                ModMask::M2,
                ModMask::LOCK | ModMask::M2,
            ] {
                self.connection
                    .grab_key(
                        false,
                        self.root,
                        mask | extra,
                        code,
                        GrabMode::ASYNC,
                        GrabMode::ASYNC,
                    )
                    .map_err(|error| error.to_string())?
                    .check()
                    .map_err(|error| format!("Cannot register {}: {error}", binding.label()))?;
            }
            self.registered.insert((code, u16::from(mask)), action);
        }
        self.connection.flush().map_err(|error| error.to_string())
    }

    fn clear(&mut self) {
        let _ = self.connection.ungrab_key(0u8, self.root, ModMask::ANY);
        self.registered.clear();
        let _ = self.connection.flush();
    }

    fn next_action(&self) -> Result<Option<Action>, String> {
        match self
            .connection
            .poll_for_event()
            .map_err(|error| error.to_string())?
        {
            Some(Event::KeyPress(event)) => {
                let modifiers = u16::from(event.state) & !(u16::from(ModMask::LOCK | ModMask::M2));
                Ok(self.registered.get(&(event.detail, modifiers)).copied())
            }
            _ => Ok(None),
        }
    }
}

impl Drop for X11State {
    fn drop(&mut self) {
        self.clear();
    }
}

fn modifier_mask(modifiers: gdk::ModifierType) -> ModMask {
    let mut mask = ModMask::from(0u16);
    if modifiers.contains(gdk::ModifierType::SHIFT_MASK) {
        mask |= ModMask::SHIFT;
    }
    if modifiers.contains(gdk::ModifierType::CONTROL_MASK) {
        mask |= ModMask::CONTROL;
    }
    if modifiers.contains(gdk::ModifierType::ALT_MASK) {
        mask |= ModMask::M1;
    }
    if modifiers.contains(gdk::ModifierType::SUPER_MASK) {
        mask |= ModMask::M4;
    }
    mask
}

#[derive(Clone)]
pub struct ShortcutManager(Rc<RefCell<ShortcutState>>);

struct ShortcutState {
    backend: Backend,
    settings: ShortcutSettings,
    generation: u64,
    x11: Option<X11State>,
    on_action: Rc<dyn Fn(Action)>,
    status: String,
    portal_session: Option<Rc<Session<GlobalShortcuts>>>,
    approved: HashMap<Action, String>,
    permission: PermissionState,
    permission_changed: Vec<Rc<dyn Fn(PermissionState)>>,
}

impl ShortcutManager {
    pub fn new(backend: Backend, on_action: impl Fn(Action) + 'static) -> Self {
        let settings = ShortcutSettings::load();
        let restore_permission =
            backend == Backend::Wayland && ShortcutSettings::permission_was_granted();
        let (x11, status) = if backend == Backend::X11 {
            match X11State::new() {
                Ok(state) => (Some(state), String::new()),
                Err(error) => (None, error),
            }
        } else {
            (None, String::new())
        };
        let manager = Self(Rc::new(RefCell::new(ShortcutState {
            backend,
            settings,
            generation: 0,
            x11,
            on_action: Rc::new(on_action),
            status,
            portal_session: None,
            approved: HashMap::new(),
            permission: if backend == Backend::Wayland {
                PermissionState::NotRequested
            } else {
                PermissionState::Granted
            },
            permission_changed: Vec::new(),
        })));
        if backend == Backend::X11 || restore_permission || manager.settings().0.is_empty() {
            manager.register();
        }
        manager
    }

    pub fn settings(&self) -> ShortcutSettings {
        self.0.borrow().settings.clone()
    }
    pub fn status(&self) -> String {
        self.0.borrow().status.clone()
    }

    pub fn display_label(&self, action: Action) -> String {
        let state = self.0.borrow();
        state.approved.get(&action).cloned().unwrap_or_else(|| {
            state
                .settings
                .get(action)
                .map(Binding::label)
                .unwrap_or_else(|| "Assign".into())
        })
    }

    pub fn is_wayland(&self) -> bool {
        self.0.borrow().backend == Backend::Wayland
    }

    pub fn permission_state(&self) -> PermissionState {
        self.0.borrow().permission.clone()
    }

    pub fn connect_permission_changed(&self, callback: impl Fn(PermissionState) + 'static) {
        let callback = Rc::new(callback);
        callback(self.permission_state());
        self.0.borrow_mut().permission_changed.push(callback);
    }

    fn set_permission_state(&self, permission: PermissionState) {
        let listeners = {
            let mut state = self.0.borrow_mut();
            state.permission = permission.clone();
            state.permission_changed.clone()
        };
        for listener in listeners {
            listener(permission.clone());
        }
    }

    pub fn request_permission(&self) {
        if !self.is_wayland()
            || matches!(
                self.permission_state(),
                PermissionState::Requesting | PermissionState::Granted
            )
        {
            return;
        }
        self.register();
    }

    pub fn configure_desktop(&self, done: impl Fn(Result<(), String>) + 'static) {
        let session = self.0.borrow().portal_session.clone();
        let manager = self.clone();
        glib::MainContext::default().spawn_local(async move {
            let result = async {
                let session = session.ok_or("Global shortcuts are still connecting")?;
                let portal = GlobalShortcuts::new()
                    .await
                    .map_err(|error| error.to_string())?;
                portal
                    .configure_shortcuts(&session, None, ConfigureShortcutsOptions::default())
                    .await
                    .map_err(|error| error.to_string())?;
                let response = portal
                    .list_shortcuts(&session, Default::default())
                    .await
                    .map_err(|error| error.to_string())?
                    .response()
                    .map_err(|error| error.to_string())?;
                manager.set_approved(response.shortcuts());
                Ok(())
            }
            .await;
            done(result);
        });
    }

    pub fn update(&self, action: Action, binding: Option<Binding>) -> Result<(), String> {
        if self.is_wayland() && !self.permission_state().is_granted() {
            return Err("Enable global shortcuts before editing bindings".into());
        }
        let mut candidate = self.settings();
        candidate.set(action, binding)?;
        {
            let mut state = self.0.borrow_mut();
            let previous = state.settings.clone();
            if let Some(x11) = &mut state.x11
                && let Err(error) = x11.register(&candidate)
            {
                let _ = x11.register(&previous);
                return Err(error);
            }
        }
        if let Err(error) = candidate.save() {
            let mut state = self.0.borrow_mut();
            let previous = state.settings.clone();
            if let Some(x11) = &mut state.x11 {
                let _ = x11.register(&previous);
            }
            return Err(error);
        }
        self.0.borrow_mut().settings = candidate;
        self.0.borrow_mut().approved.clear();
        if self.0.borrow().backend == Backend::Wayland {
            self.register();
        }
        Ok(())
    }

    fn register(&self) {
        let mut state = self.0.borrow_mut();
        state.generation += 1;
        state.portal_session = None;
        state.approved.clear();
        let generation = state.generation;
        state.status.clear();
        match state.backend {
            Backend::X11 => {
                let settings = state.settings.clone();
                if let Some(x11) = &mut state.x11
                    && let Err(error) = x11.register(&settings)
                {
                    state.status = error;
                }
            }
            Backend::Wayland => {
                let settings = state.settings.clone();
                let manager = self.clone();
                drop(state);
                if settings.0.is_empty() {
                    manager.set_permission_state(PermissionState::Granted);
                    return;
                }
                manager.set_permission_state(PermissionState::Requesting);
                glib::MainContext::default().spawn_local(async move {
                    if generation > 1 {
                        glib::timeout_future(std::time::Duration::from_millis(250)).await;
                    }
                    if manager.0.borrow().generation != generation {
                        return;
                    }
                    if let Err(error) = manager.portal_loop(settings, generation).await
                        && manager.0.borrow().generation == generation
                    {
                        manager.0.borrow_mut().status = error.clone();
                        if let Err(save_error) = ShortcutSettings::save_permission_granted(false) {
                            eprintln!("Could not save shortcut permission state: {save_error}");
                        }
                        manager.set_permission_state(PermissionState::Denied(error));
                    }
                });
            }
            Backend::Unknown => {}
        }
    }

    fn set_approved(&self, shortcuts: &[ashpd::desktop::global_shortcuts::Shortcut]) {
        let mut state = self.0.borrow_mut();
        state.approved.clear();
        for shortcut in shortcuts {
            if let Some(action) = Action::ALL
                .into_iter()
                .find(|action| action.id() == shortcut.id())
                && !shortcut.trigger_description().is_empty()
            {
                state
                    .approved
                    .insert(action, shortcut.trigger_description().to_owned());
            }
        }
    }

    async fn portal_loop(&self, settings: ShortcutSettings, generation: u64) -> Result<(), String> {
        if settings.0.is_empty() {
            return Ok(());
        }
        let portal = GlobalShortcuts::new()
            .await
            .map_err(|error| error.to_string())?;
        let session = Rc::new(
            portal
                .create_session(CreateSessionOptions::default())
                .await
                .map_err(|error| error.to_string())?,
        );
        let shortcuts: Vec<_> = Action::ALL
            .iter()
            .filter_map(|action| {
                settings.get(*action).map(|binding| {
                    NewShortcut::new(action.id(), action.label())
                        .preferred_trigger(Some(binding.portal_trigger().as_str()))
                })
            })
            .collect();
        let registration = async {
            let activated = portal
                .receive_activated()
                .await
                .map_err(|error| error.to_string())?;
            let response = portal
                .bind_shortcuts(&session, &shortcuts, None, BindShortcutsOptions::default())
                .await
                .map_err(|error| error.to_string())?
                .response()
                .map_err(|error| error.to_string())?;
            Ok::<_, String>((activated, response))
        }
        .await;
        let (mut activated, response) = match registration {
            Ok(value) => value,
            Err(error) => {
                let _ = session.close().await;
                return Err(error);
            }
        };
        if response.shortcuts().is_empty() {
            let _ = session.close().await;
            return Err("No global shortcuts were approved by the desktop".into());
        }
        if self.0.borrow().generation == generation {
            self.0.borrow_mut().portal_session = Some(session.clone());
            self.set_approved(response.shortcuts());
            if let Err(error) = ShortcutSettings::save_permission_granted(true) {
                eprintln!("Could not save shortcut permission state: {error}");
            }
            self.set_permission_state(PermissionState::Granted);
        }
        loop {
            if self.0.borrow().generation != generation {
                break;
            }
            let event = activated.next();
            let timeout = glib::timeout_future(std::time::Duration::from_millis(200));
            match futures_util::future::select(Box::pin(event), Box::pin(timeout)).await {
                Either::Left((Some(event), _)) => {
                    if let Some(action) = Action::ALL
                        .into_iter()
                        .find(|action| action.id() == event.shortcut_id())
                    {
                        (self.0.borrow().on_action)(action);
                    }
                }
                Either::Left((None, _)) => break,
                Either::Right(_) => {}
            }
        }
        if self.0.borrow().generation == generation {
            self.0.borrow_mut().portal_session = None;
        }
        let _ = session.close().await;
        Ok(())
    }

    pub fn start_x11_poll(&self) {
        if self.0.borrow().backend != Backend::X11 {
            return;
        }
        let manager = self.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(20), move || {
            let action = {
                let state = manager.0.borrow();
                state
                    .x11
                    .as_ref()
                    .and_then(|x11| x11.next_action().ok().flatten())
            };
            if let Some(action) = action {
                (manager.0.borrow().on_action)(action);
            }
            glib::ControlFlow::Continue
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, Binding, ShortcutSettings};
    use gtk4::{gdk, glib};

    #[test]
    fn defaults_and_duplicate_bindings() {
        let mut settings = ShortcutSettings::default();
        assert_eq!(settings.get(Action::Computer).unwrap().key, gdk::Key::c);
        assert_eq!(
            settings.get(Action::Computer).unwrap().portal_trigger(),
            "ALT+c"
        );
        assert_eq!(settings.get(Action::Pen).unwrap().key, gdk::Key::b);
        assert_eq!(settings.get(Action::Pen).unwrap().portal_trigger(), "ALT+b");
        assert_eq!(settings.get(Action::Eraser).unwrap().key, gdk::Key::e);
        assert_eq!(
            settings.get(Action::Eraser).unwrap().portal_trigger(),
            "ALT+e"
        );
        assert!(settings.get(Action::Clear).is_none());
        assert!(
            settings
                .set(
                    Action::Clear,
                    Some(Binding {
                        key: gdk::Key::c,
                        modifiers: gdk::ModifierType::ALT_MASK,
                    })
                )
                .is_err()
        );
        settings.set(Action::Computer, None).unwrap();
        assert!(settings.get(Action::Computer).is_none());
    }

    #[test]
    fn settings_round_trip_preserves_display_preferences() {
        let path =
            std::env::temp_dir().join(format!("painto-shortcuts-test-{}.ini", std::process::id()));
        let settings = ShortcutSettings::default();
        settings.save_to(&path).unwrap();
        ShortcutSettings::save_permission_granted_to(&path, true).unwrap();
        assert!(ShortcutSettings::permission_was_granted_at(&path));
        let file = glib::KeyFile::new();
        file.load_from_file(&path, glib::KeyFileFlags::NONE)
            .unwrap();
        file.set_boolean("Canvas", "all-monitors", true);
        file.save_to_file(&path).unwrap();
        let mut saved = ShortcutSettings::load_from(&path);
        saved
            .set(
                Action::Clear,
                Some(Binding {
                    key: gdk::Key::k,
                    modifiers: gdk::ModifierType::CONTROL_MASK,
                }),
            )
            .unwrap();
        saved.save_to(&path).unwrap();
        let reloaded = ShortcutSettings::load_from(&path);
        assert_eq!(reloaded.get(Action::Clear).unwrap().name(), "Ctrl+k");
        assert!(reloaded.get(Action::Close).is_none());
        let file = glib::KeyFile::new();
        file.load_from_file(&path, glib::KeyFileFlags::NONE)
            .unwrap();
        assert!(file.boolean("Canvas", "all-monitors").unwrap());
        assert!(ShortcutSettings::permission_was_granted_at(&path));
        std::fs::remove_file(path).unwrap();
    }
}
