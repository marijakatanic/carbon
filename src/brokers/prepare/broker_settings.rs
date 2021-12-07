use crate::data::SpongeSettings;

use std::time::Duration;

#[derive(Debug, Clone)]
pub(crate) struct BrokerSettings {
    pub brokerage_sponge_settings: SpongeSettings,
    pub reduction_timeout: Option<Duration>,
    pub ping_interval: Duration,
    pub fast_witness_timeout: Duration,
}

impl Default for BrokerSettings {
    fn default() -> Self {
        BrokerSettings {
            brokerage_sponge_settings: Default::default(),
            reduction_timeout: Some(Duration::from_secs(1)),
            ping_interval: Duration::from_secs(60),
            fast_witness_timeout: Duration::from_secs(1),
        }
    }
}
