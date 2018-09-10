use config::{Config, ConfigError, File, FileFormat};
use helpers::{get_kbd_input, get_keyboard, get_display};
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Paths {
    pub keyboard_backlight: String,
    pub keyboard_input: String,
    pub display: String,
    pub trackpad_input: String,
    pub illuminance: String,
    pub sound_card: String,
}

#[derive(Debug, Deserialize)]
pub struct Preferences {
    pub idle_timeout: u64,
    pub tick_time: u64,
    pub dim_percent: f32,
    pub keyboard_steps: u32,
    pub display_steps: u32
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub paths: Paths,
    pub preferences: Preferences
}


impl Settings {
    pub fn new(p: &str) -> Result<Self, ConfigError> {
        let mut s = Config::new();

        s.set_default("paths.display", get_display())?;
        s.set_default("paths.keyboard_backlight", get_keyboard())?;
        s.set_default("paths.keyboard_input", get_kbd_input())?;
        s.set_default("paths.trackpad_input", String::from("/dev/input/mouse1"))?;
        s.set_default("paths.illuminance", String::from("/sys/bus/iio/devices/iio:device0/in_illuminance_raw"))?;
        s.set_default("paths.sound_card", String::from("/proc/asound/card1/pcm0p/sub0/status"))?;

        s.set_default("preferences.idle_timeout", 60)?;
        s.set_default("preferences.tick_time", 5)?;
        s.set_default("preferences.dim_percent", 0.6)?;
        s.set_default("preferences.display_steps", 10)?;
        s.set_default("preferences.keyboard_steps", 10)?;


        if Path::new(p).is_file() {
            s.merge(File::new(p, FileFormat::Ini))?;
        }

        s.try_into()
    }
}
