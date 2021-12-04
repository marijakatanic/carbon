use std::time::Duration;

#[derive(Debug, Clone)]
pub(crate) struct SpongeSettings {
    pub capacity: usize,
    pub timeout: Option<Duration>,
}

impl Default for SpongeSettings {
    fn default() -> Self {
        SpongeSettings {
            capacity: 100,
            timeout: Some(Duration::from_secs(1)),
        }
    }
}
