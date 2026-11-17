use std::{fs, path::PathBuf};

use gtk4::glib::{self, KeyFile, KeyFileFlags};

#[derive(Clone, Default)]
pub struct DisplaySettings {
    pub connector: String,
    pub all_monitors: bool,
}

impl DisplaySettings {
    fn path() -> PathBuf {
        glib::user_config_dir().join("painto").join("settings.ini")
    }

    pub fn load() -> Self {
        Self::load_from(&Self::path())
    }

    fn load_from(path: &std::path::Path) -> Self {
        let file = KeyFile::new();
        if file.load_from_file(path, KeyFileFlags::NONE).is_err() {
            return Self::default();
        }
        Self {
            connector: file
                .string("Canvas", "monitor")
                .map(|value| value.to_string())
                .unwrap_or_default(),
            all_monitors: file.boolean("Canvas", "all-monitors").unwrap_or(false),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        self.save_to(&Self::path())
    }

    fn save_to(&self, path: &std::path::Path) -> Result<(), String> {
        fs::create_dir_all(path.parent().ok_or("Invalid settings path")?)
            .map_err(|error| error.to_string())?;
        let file = KeyFile::new();
        file.set_string("Canvas", "monitor", &self.connector);
        file.set_boolean("Canvas", "all-monitors", self.all_monitors);
        file.save_to_file(path).map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::DisplaySettings;

    #[test]
    fn settings_round_trip() {
        let path =
            std::env::temp_dir().join(format!("painto-settings-test-{}.ini", std::process::id()));
        let expected = DisplaySettings {
            connector: "HDMI-A-1".into(),
            all_monitors: true,
        };
        expected.save_to(&path).unwrap();
        let actual = DisplaySettings::load_from(&path);
        assert_eq!(actual.connector, expected.connector);
        assert_eq!(actual.all_monitors, expected.all_monitors);
        std::fs::remove_file(path).unwrap();
    }
}
