use std::time::Duration;

#[derive(Debug, Clone)]
pub(crate) struct SpongeSettings {
    pub capacity: usize,
    pub timeout: Duration,
}

impl Default for SpongeSettings {
    fn default() -> Self {
        SpongeSettings {
            capacity: 50000,
            timeout: Duration::from_secs(1),
        }
    }
}
