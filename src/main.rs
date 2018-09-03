extern crate libc;

mod input;

use input::{is_key_event, is_key_press, InputEvent};

use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::mem;
use std::thread;
// use std::path::Path
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

enum Message {
    Tick,
    GeneralEvent,
    KeyboardBrightnessUp,
    KeyboardBrightnessDown,
    DisplayBrightnessDown,
    DisplayBrightnessUp,
}

struct BrightnessValues {
    display: f32,
    keyboard: f32,
}

fn sensor_to_range(sensor_value: u32) -> u32 {
    match sensor_value {
        0...20 => 1,
        20...50 => 2,
        50...100 => 3,
        100...200 => 4,
        200...300 => 5,
        _ => 6
    }
}

fn map_sensor_range_to_bl_vals(sensor_range: u32) -> BrightnessValues {
    match sensor_range {
        1 => BrightnessValues { keyboard: 0.1, display: 0.15},
        2 => BrightnessValues { keyboard: 0.0, display: 0.20},
        3 => BrightnessValues { keyboard: 0.0, display: 0.25},
        4 => BrightnessValues { keyboard: 0.0, display: 0.30},
        5 => BrightnessValues { keyboard: 0.0, display: 0.35},
        _ => BrightnessValues { keyboard: 0.0, display: 0.40},
    }
}

fn read_file_to_string(filename: &str) -> std::io::Result<String> {
    println!("Reading file: {}", filename);
    let mut fd = File::open(filename)?;
    let mut s = String::new();
    fd.read_to_string(&mut s)?;
    Ok(s)
}

fn read_file_to_u32(filename: &str) -> Option<u32> {
    read_file_to_string(filename)
        .map_err(|e| {
            panic!("Cannot read file `{}`: {}", filename, e);
        })
        .ok()
        .and_then(|s| {
            s.trim_right()
                .parse::<u32>()
                .map_err(|e| format!("Cannot parse {} as integer: {} from `{}`", s, e, filename))
                .ok()
        })
}

fn write_u32_to_file(filename: &str, value: u32) -> std::io::Result<()> {
    println!("Writing file: {}", filename);
    OpenOptions::new()
        .write(true)
        .open(filename)
        .and_then(|mut fs| fs.write_all(value.to_string().as_ref()))
        .map_err(|e| {
            panic!("Cannot write to file `{}` error: {}", filename, e);
        })
}

fn keyboard_input() -> String {
    let mut command_str = "grep -E 'Handlers|EV' /proc/bus/input/devices".to_string();
    command_str.push_str("| grep -B1 120013");
    command_str.push_str("| grep -Eo event[0-9]+");

    let res = Command::new("sh").arg("-c").arg(command_str).output().unwrap();
    let res_str = std::str::from_utf8(&res.stdout).unwrap();
    let mut filename = "/dev/input/".to_string();
    filename.push_str(res_str.trim());
    filename
}

fn get_backlight_device(dir:&str, name: &str) -> String {
    let command_str = format!("find /sys/class/{} -mindepth 1 -name '*{}'", dir, name);

    let res = Command::new("sh").arg("-c").arg(command_str).output().unwrap();
    let res_str = std::str::from_utf8(&res.stdout).unwrap();
    res_str.trim().to_string()
}

fn keyboard_backlight() -> String {
    get_backlight_device("leds", "kbd_backlight")
}
fn display_backlight() -> String {
    get_backlight_device("backlight", "backlight")
}

fn get_max_brightness(dir: String) -> Option<u32> {
    let mut filename = dir;
    filename.push_str("/max_brightness");
    read_file_to_u32(&filename)
}

fn get_brightness_file(dir: &str) -> String {
    format!("{}/brightness", dir)
}

fn media_playing() -> bool {
    let command_str = "cat /proc/asound/card1/pcm0p/sub0/status | grep -w state | grep -q RUNNING";
    let res = Command::new("sh").arg("-c").arg(command_str).status().unwrap();
    res.success()
}

fn mult(a: f32, b: u32) -> u32 {
    let x = a * b as f32;
    x as u32
}

fn run_as_root() -> bool {
    let euid = unsafe { libc::getuid() };
    euid == 0
}

fn main() {

    // initial set up stuff
    if !run_as_root() {
        panic!("Must be run as root.");
    }

    // TODO: try to get everything automatically, instead of hard-coding
    let display_backlight = display_backlight();
    let keyboard_backlight = keyboard_backlight();
    let keyboard_input = keyboard_input();

    // although these are usually in the same spot I think?
    let trackpad_input = "/dev/input/mouse1";
    let als = "/sys/bus/iio/devices/iio:device0/in_illuminance_raw";

    // Brightness file setup
    let display_backlight_file = get_brightness_file(&display_backlight);
    let keyboard_backlight_file = get_brightness_file(&keyboard_backlight);

    // Construct initial state
    let keyboard_max_brightness = get_max_brightness(keyboard_backlight).expect("Error getting max brightness of keyboard leds");
    let display_max_brightness = get_max_brightness(display_backlight).expect("Error getting max brightness of backlight");
    let mut sensor_range = sensor_to_range(read_file_to_u32(&als).expect("Error reading ambient light sensor"));

    let keyboard_step = keyboard_max_brightness / 10;
    let display_step = display_max_brightness / 15;
    let (tx, rx) = mpsc::channel();

    let mut idle = false;
    let mut idle_time = 0;
    let idle_timeout = 60;
    let tick_time = 5;
    let mut keyboard_override = false;
    let mut display_override = false;
    let mut current_keyboard_brightness = read_file_to_u32(&keyboard_backlight_file).expect("Error reading keyboard brightness");
    let mut current_display_brightness = read_file_to_u32(&display_backlight_file).expect("Error reading display brightness");

    // Timer thread
    let timer_tx = mpsc::Sender::clone(&tx);
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(tick_time));
            timer_tx.send(Message::Tick).unwrap();
        }
    });

    // Keyboard watcher thread
    let keyboard_tx = mpsc::Sender::clone(&tx);
    thread::spawn(move || {
        let mut device_file = File::open(keyboard_input).expect("Couldn't read keyboard");
        let mut buf: [u8; 24] = unsafe { mem::zeroed() };
        loop {
            let num_bytes = device_file.read(&mut buf).expect("!!");
            if num_bytes != mem::size_of::<InputEvent>() {
                panic!("Error while reading from device");
            }
            let event: InputEvent = unsafe { mem::transmute(buf) };
            if is_key_event(event.type_) && is_key_press(event.value) {
                match event.code {
                    224 => keyboard_tx.send(Message::DisplayBrightnessDown).unwrap(),
                    225 => keyboard_tx.send(Message::DisplayBrightnessUp).unwrap(),
                    229 => keyboard_tx.send(Message::KeyboardBrightnessDown).unwrap(),
                    230 => keyboard_tx.send(Message::KeyboardBrightnessUp).unwrap(),
                    _  =>  keyboard_tx.send(Message::GeneralEvent).unwrap(),
                }
            };
        }
    });

    // Trackpad watcher thread
    let trackpad_tx = mpsc::Sender::clone(&tx);
    thread::spawn(move || {
        let mut dev_file = File::open(trackpad_input).expect("Could not read mouse");
        let mut buf: [u8; 24] = unsafe { mem::zeroed() };
        while dev_file.read(&mut buf).expect("!!") > 0 {
            trackpad_tx.send(Message::GeneralEvent).unwrap();
        }
    });

    for msg in rx {
        match msg {
            Message::DisplayBrightnessDown => {
                current_display_brightness = current_display_brightness.saturating_sub(display_step);
                write_u32_to_file(&display_backlight_file, current_display_brightness).expect("Failed to write file");
                idle = false;
                idle_time = 0;
                display_override = true;
            }
            Message::DisplayBrightnessUp => {
                current_display_brightness = if current_display_brightness + display_step > display_max_brightness { display_max_brightness } else { current_display_brightness + display_step };
                write_u32_to_file(&display_backlight_file, current_display_brightness).expect("Failed to write file");
                idle = false;
                idle_time = 0;
                display_override = true;
            }
            Message::KeyboardBrightnessDown => {
                current_keyboard_brightness = current_keyboard_brightness.saturating_sub(keyboard_step);
                write_u32_to_file(&keyboard_backlight_file, current_keyboard_brightness).expect("Failed to write file");
                idle = false;
                idle_time = 0;
                keyboard_override = true;
            }
            Message::KeyboardBrightnessUp => {
                current_keyboard_brightness = if current_keyboard_brightness + keyboard_step > keyboard_max_brightness { keyboard_max_brightness } else { current_keyboard_brightness + keyboard_step };
                write_u32_to_file(&keyboard_backlight_file, current_keyboard_brightness).expect("Failed to write file");
                idle = false;
                idle_time = 0;
                keyboard_override = true;
            }
            Message::GeneralEvent => {
                if idle {
                    // Restore old values
                    write_u32_to_file(&display_backlight_file, current_display_brightness).expect("Failed to write file");
                    write_u32_to_file(&keyboard_backlight_file, current_keyboard_brightness).expect("Failed to write file");
                    idle = false;
                    idle_time = 0;
                } else {
                    idle_time = 0;
                }
            }
            Message::Tick => {
                // active -> idle
                if !idle && idle_time > idle_timeout && !media_playing() {
                    // Dim the screen and turn off kb backlight
                    write_u32_to_file(&display_backlight_file, mult(0.6, current_display_brightness)).expect("Failed to write file");
                    write_u32_to_file(&keyboard_backlight_file, 0).expect("Failed to write file");
                    idle = true;
                } else if !idle {
                    let new_sensor_range = sensor_to_range(read_file_to_u32(&als).expect("Error reading ambient light sensor"));
                    if new_sensor_range != sensor_range {
                        let vals = map_sensor_range_to_bl_vals(new_sensor_range);
                        if !display_override {
                            write_u32_to_file(&display_backlight_file, mult(vals.display, display_max_brightness)).expect("Failed to write file");
                        }
                        if !keyboard_override {
                            write_u32_to_file(&keyboard_backlight_file, mult(vals.keyboard, keyboard_max_brightness)).expect("Failed to write file");
                        }
                        sensor_range = new_sensor_range;
                    }
                }
                idle_time += tick_time;
            }
        }
    }
}
