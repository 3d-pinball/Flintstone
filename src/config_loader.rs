#[derive(Debug)]
pub struct ConfigLoader {
    pub max_flight_frames: u32,
    pub enable_sampler_anisotropy: bool,
}

impl ConfigLoader {
    pub fn new() -> Self {
        Self {
            max_flight_frames: 2,
            enable_sampler_anisotropy: true,
        }
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}
