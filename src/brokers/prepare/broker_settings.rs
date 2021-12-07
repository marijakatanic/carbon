use crate::data::SpongeSettings;

use std::time::Duration;

#[derive(Debug, Clone)]
pub(crate) struct BrokerSettings {
    pub brokerage_sponge_settings: SpongeSettings,

    pub reduction_timeout: Duration,
    pub optimistic_witness_timeout: Duration,

    pub ping_interval: Duration,
}

pub(in crate::brokers::prepare) struct BrokerSettingsComponents {
    pub flush: FlushTaskSettings,
    pub broker: BrokerTaskSettings,
    pub ping: PingTaskSettings,
}
#[derive(Debug, Clone)]
pub(in crate::brokers::prepare) struct FlushTaskSettings {
    pub brokerage_sponge_settings: SpongeSettings,
}

#[derive(Debug, Clone)]
pub(in crate::brokers::prepare) struct BrokerTaskSettings {
    pub reduction_timeout: Duration,
    pub optimistic_witness_timeout: Duration,
}

#[derive(Debug, Clone)]
pub(in crate::brokers::prepare) struct PingTaskSettings {
    pub ping_interval: Duration,
}

impl BrokerSettings {
    pub(in crate::brokers::prepare) fn into_components(self) -> BrokerSettingsComponents {
        BrokerSettingsComponents {
            flush: FlushTaskSettings {
                brokerage_sponge_settings: self.brokerage_sponge_settings,
            },
            broker: BrokerTaskSettings {
                reduction_timeout: self.reduction_timeout,
                optimistic_witness_timeout: self.optimistic_witness_timeout,
            },
            ping: PingTaskSettings {
                ping_interval: self.ping_interval,
            },
        }
    }
}

impl Default for BrokerSettings {
    fn default() -> Self {
        BrokerSettings {
            brokerage_sponge_settings: Default::default(),

            reduction_timeout: Duration::from_secs(1),
            optimistic_witness_timeout: Duration::from_secs(1),

            ping_interval: Duration::from_secs(60),
        }
    }
}
