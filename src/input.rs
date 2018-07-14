const EV_KEY: u16 = 1;
const KEY_PRESS: i32 = 1;

#[repr(C)]
pub struct InputEvent {
    tv_sec: isize,
    tv_usec: isize,
    pub type_: u16,
    pub code: u16,
    pub value: i32
}

pub fn is_key_event(type_: u16) -> bool {
    type_ == EV_KEY
}

pub fn is_key_press(value: i32) -> bool {
    value == KEY_PRESS
}
