#[macro_use]
extern crate serde_derive;
extern crate config;
extern crate docopt;
extern crate libc;

mod app;
mod helpers;
mod input;
mod settings;
mod types;

use docopt::Docopt;
use helpers::{build_device, version, run_as_root};
use settings::Settings;
use types::Args;

const USAGE: &'static str = "
laptop-lights

Usage:
  laptop-lights [--config=<file>]
  laptop-lights (-h | --help)
  laptop-lights --version

Options:
  -h --help         Show this screen.
  --version         Show version.
  --config=<file>   Path to config file.
";

fn main() {

    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.version(Some(version()))
                       .deserialize())
        .unwrap_or_else(|e| e.exit());

    if !run_as_root() {
        panic!("Must be run as root.");
    }

    let conf_file = match args.flag_config {
        Some(x) => x,
        None => String::from("/etc/laptop-lights.conf")
    };

    let settings = Settings::new(&conf_file).unwrap();

    let kbd = build_device(&settings.paths.keyboard_backlight, settings.preferences.keyboard_steps);
    let dsp = build_device(&settings.paths.display, settings.preferences.display_steps);

    app::run(settings, kbd, dsp);

}
