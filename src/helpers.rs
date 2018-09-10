use libc;
use std::fs::{File, OpenOptions};
use std::io::{Read, Result, Write};
use std::process::Command;
use types::BacklightDevice;

pub fn als_to_kb(als: u32, kbd_max: u32) -> u32 {
    if als < 5 {
        (0.1 * kbd_max as f32) as u32
    } else {
        0
    }
}

pub fn als_to_dsp(als: u32, bl_max: u32) -> u32 {
    let als_bl_max = 0.6 * bl_max as f32;
    (0.15 * als_bl_max as f32 + (als as f32 / 500.0 * (0.85 * als_bl_max as f32))) as u32
}

pub fn read_file_to_string(p: &str) -> Result<String> {
    let mut fd = File::open(p)?;
    let mut s = String::new();
    fd.read_to_string(&mut s)?;
    Ok(s)
}

pub fn read_file_to_u32(p: &str) -> Option<u32> {
    read_file_to_string(p)
        .map_err(|e| {
            panic!("Cannot read file `{:?}`: {}", p, e);
        })
        .ok()
        .and_then(|s| {
            s.trim_right()
                .parse::<u32>()
                .map_err(|e| format!("Cannot parse {} as integer: {} from `{:?}`", s, e, p))
                .ok()
        })
}

pub fn write_u32_to_file(filename: &str, value: u32) -> Result<()> {
    println!("Writing file: {}", filename);
    OpenOptions::new()
        .write(true)
        .open(filename)
        .and_then(|mut fs| fs.write_all(value.to_string().as_ref()))
        .map_err(|e| {
            panic!("Cannot write to file `{}` error: {}", filename, e);
        })
}

pub fn get_kbd_input() -> String {
    let mut command_str = "grep -E 'Handlers|EV' /proc/bus/input/devices".to_string();
    command_str.push_str("| grep -B1 120013");
    command_str.push_str("| grep -Eo event[0-9]+");

    let res = Command::new("sh")
        .arg("-c")
        .arg(command_str)
        .output()
        .unwrap();
    let res_str = String::from_utf8(res.stdout).unwrap();
    let path = String::from("/dev/input/");

    path + res_str.trim()
}

fn get_backlight_device(dir: &str, name: &str) -> String {
    let command_str = format!("find /sys/class/{} -mindepth 1 -name '*{}'", dir, name);

    let res = Command::new("sh")
        .arg("-c")
        .arg(command_str)
        .output()
        .unwrap();
    let res_str = String::from_utf8(res.stdout).unwrap();
    res_str.trim().to_string()
}

pub fn get_keyboard() -> String {
    get_backlight_device("leds", "kbd_backlight")
}
pub fn get_display() -> String {
    get_backlight_device("backlight", "backlight")
}

pub fn get_max(dir: &str) -> u32 {
    let mut filename = String::from(dir);
    filename.push_str("/max_brightness");
    read_file_to_u32(&filename).unwrap()
}

pub fn get_brightness_file(dir: &str) -> String {
    format!("{}/brightness", dir)
}

pub fn build_device(dir: &str, steps: u32) -> BacklightDevice {
    let max = get_max(dir);
    BacklightDevice {
        file: get_brightness_file(&dir),
        max: max,
        step: max / steps
    }
}

pub fn media_playing(sc: &str) -> bool {
    let command_str = format!("cat {} | grep -w state | grep -q RUNNING", sc);
    let res = Command::new("sh")
        .arg("-c")
        .arg(command_str)
        .status()
        .unwrap();
    res.success()
}

pub fn mult(a: f32, b: u32) -> u32 {
    let x = a * b as f32;
    x as u32
}

pub fn step_down(val: u32, step: u32) -> u32 {
    val.saturating_sub(step)
}

pub fn step_up(val: u32, step: u32, max: u32) -> u32 {
    let new_val = val + step;
    if new_val > max {
        max
    } else {
        new_val
    }
}

pub fn run_as_root() -> bool {
    let euid = unsafe { libc::getuid() };
    euid == 0
}

pub fn version() -> String {
    let (maj, min, pat) = (
        option_env!("CARGO_PKG_VERSION_MAJOR"),
        option_env!("CARGO_PKG_VERSION_MINOR"),
        option_env!("CARGO_PKG_VERSION_PATCH"),
    );
    match (maj, min, pat) {
        (Some(maj), Some(min), Some(pat)) => format!("{}.{}.{}", maj, min, pat),
        _ => "".to_owned(),
    }
}
