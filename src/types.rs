// use std::path::Path;
#[derive(Debug, Deserialize)]
pub struct Args {
    pub flag_config: Option<String>,
}

pub struct BacklightDevice {
    pub file: String,
    pub max: u32,
    pub step: u32,
}

pub enum Message {
    Tick,
    Input,
    KbdUp,
    KbdDown,
    DspUp,
    DspDown,
}
