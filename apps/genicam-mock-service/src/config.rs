use crate::Cli;

#[derive(Debug, Clone)]
pub struct MockConfig {
    pub device_id: String,
    pub device_name: String,
    pub model: String,
    pub serial: String,
    pub width: u32,
    pub height: u32,
    pub fps: f32,
}

impl MockConfig {
    pub fn from_cli(cli: &Cli) -> Self {
        Self {
            device_id: cli.device_id.clone(),
            device_name: cli.device_name.clone(),
            model: cli.model.clone(),
            serial: cli.serial.clone(),
            width: cli.width,
            height: cli.height,
            fps: cli.fps,
        }
    }
}
