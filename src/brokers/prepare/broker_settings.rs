use crate::data::SpongeSettings;

#[derive(Debug, Clone, Default)]
pub(crate) struct BrokerSettings {
    pub brokerage_sponge_settings: SpongeSettings,
    pub reduction_sponge_settings: SpongeSettings,
}
