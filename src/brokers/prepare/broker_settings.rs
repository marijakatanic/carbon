use crate::data::SpongeSettings;

use std::time::Duration;

#[derive(Debug, Clone)]
pub(crate) struct BrokerSettings {
    pub brokerage_sponge_settings: SpongeSettings,
    pub reduction_timeout: Option<Duration>,
}

impl Default for BrokerSettings {
    fn default() -> Self {
        BrokerSettings {
            brokerage_sponge_settings: Default::default(),
            reduction_timeout: Some(Duration::from_secs(1)),
        }
    }
}
