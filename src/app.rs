use helpers::*;
use input::{is_key_event, is_key_press, InputEvent};
use settings::Settings;
use types::{BacklightDevice, Message};

use std::fs::File;
use std::io::prelude::*;
use std::mem;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub fn run(s: Settings, kbd: BacklightDevice, dsp: BacklightDevice) {

    let mut idle = false;
    let mut idle_time = 0;
    let mut als_value: u32;
    let mut dsp_val: u32;
    let mut kbd_val: u32;
    let mut kbd_override = false;
    let mut dsp_override = false;

    // Initialize everything
    als_value = read_file_to_u32(&s.paths.illuminance).unwrap();
    dsp_val = als_to_dsp(als_value, dsp.max);
    kbd_val = als_to_kb(als_value, kbd.max);

    // Write initial values
    write_u32_to_file(&dsp.file, dsp_val).unwrap();
    write_u32_to_file(&kbd.file, kbd_val).unwrap();

    // Launch out threads
    let (tx, rx) = mpsc::channel();

    start_timer(mpsc::Sender::clone(&tx), s.preferences.tick_time);
    start_keyboard_watcher(mpsc::Sender::clone(&tx), s.paths.keyboard_input);
    start_trackpad_watcher(mpsc::Sender::clone(&tx), s.paths.trackpad_input);

    for msg in rx {
        match msg {
            Message::DspDown => {
                dsp_val = step_down(dsp_val, dsp.step);
                write_u32_to_file(&dsp.file, dsp_val).expect("Failed to write file");
                idle = false;
                idle_time = 0;
                dsp_override = true;
            }
            Message::DspUp => {
                dsp_val = step_up(dsp_val, dsp.step, dsp.max);
                write_u32_to_file(&dsp.file, dsp_val).expect("Failed to write file");
                idle = false;
                idle_time = 0;
                dsp_override = true;
            }
            Message::KbdDown => {
                kbd_val = step_down(kbd_val, kbd.step);
                write_u32_to_file(&kbd.file, kbd_val).expect("Failed to write file");
                idle = false;
                idle_time = 0;
                kbd_override = true;
            }
            Message::KbdUp => {
                kbd_val = step_up(kbd_val, kbd.step, kbd.max);
                write_u32_to_file(&kbd.file, kbd_val).expect("Failed to write file");
                idle = false;
                idle_time = 0;
                kbd_override = true;
            }
            Message::Input => {
                if idle {
                    // Restore old values
                    write_u32_to_file(&dsp.file, dsp_val).expect("Failed to write file");
                    write_u32_to_file(&kbd.file, kbd_val).expect("Failed to write file");
                    idle = false;
                    idle_time = 0;
                } else {
                    idle_time = 0;
                }
            }
            Message::Tick => {
                // active -> idle
                if !idle && idle_time > s.preferences.idle_timeout && !media_playing(&s.paths.sound_card) {
                    // Dim the screen and turn off kb backlight
                    write_u32_to_file(&dsp.file, mult(s.preferences.dim_percent, dsp_val))
                        .expect("Failed to write file");
                    write_u32_to_file(&kbd.file, 0).expect("Failed to write file");
                    idle = true;
                } else if !idle {
                    let new_als_value = read_file_to_u32(&s.paths.illuminance)
                        .expect("Error reading ambient light sensor");
                    if new_als_value != als_value {
                        als_value = new_als_value;
                        if !dsp_override {
                            let new_dsp_val = als_to_dsp(als_value, dsp.max);
                            if new_dsp_val != dsp_val {
                                dsp_val = new_dsp_val;
                                write_u32_to_file(&dsp.file, dsp_val)
                                    .expect("Failed to write file");
                            }
                        }
                        if !kbd_override {
                            let new_kbd_val = als_to_kb(als_value, kbd.max);
                            if new_kbd_val != kbd_val {
                                kbd_val = new_kbd_val;
                                write_u32_to_file(&kbd.file, kbd_val)
                                    .expect("Failed to write file");
                            }
                        }
                    }
                }
                idle_time += s.preferences.tick_time;
            }
        }
    }
}

pub fn start_timer(tx: mpsc::Sender<Message>, interval: u64) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(interval));
        tx.send(Message::Tick).unwrap();
    });
}

pub fn start_keyboard_watcher(tx: mpsc::Sender<Message>, kbd_input: String) {
    thread::spawn(move || {
        let mut dev = File::open(kbd_input).expect("Couldn't read keyboard");
        let mut buf: [u8; 24] = unsafe { mem::zeroed() };
        loop {
            let num_bytes = dev.read(&mut buf).expect("!!");
            if num_bytes != mem::size_of::<InputEvent>() {
                panic!("Error while reading from device");
            }
            let event: InputEvent = unsafe { mem::transmute(buf) };
            if is_key_event(event.type_) && is_key_press(event.value) {
                match event.code {
                    224 => tx.send(Message::DspDown).unwrap(),
                    225 => tx.send(Message::DspUp).unwrap(),
                    229 => tx.send(Message::KbdDown).unwrap(),
                    230 => tx.send(Message::KbdUp).unwrap(),
                    _ => tx.send(Message::Input).unwrap(),
                }
            };
        }
    });
}
pub fn start_trackpad_watcher(tx: mpsc::Sender<Message>, tp_input: String) {
    thread::spawn(move || {
        let mut dev_file = File::open(tp_input).expect("Could not read mouse");
        let mut buf: [u8; 24] = unsafe { mem::zeroed() };
        while dev_file.read(&mut buf).expect("!!") > 0 {
            tx.send(Message::Input).unwrap();
        }
    });
}
