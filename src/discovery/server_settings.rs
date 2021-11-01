#[derive(Debug, Clone)]
pub(crate) struct ServerSettings {
    pub install_channel_capacity: usize,
    pub frame_channel_capacity: usize,
}

impl Default for ServerSettings {
    fn default() -> Self {
        ServerSettings {
            install_channel_capacity: 32,
            frame_channel_capacity: 32,
        }
    }
}
