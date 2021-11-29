use crate::data::SpongeSettings;

#[derive(Debug, Clone, Default)]
pub(crate) struct BrokerSettings {
    pub prepare_settings: PrepareSettings,
    pub request_sponge_settings: SpongeSettings,
    pub reduction_sponge_settings: SpongeSettings,
}
