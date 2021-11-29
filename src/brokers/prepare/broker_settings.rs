use crate::data::SpongeSettings;

#[derive(Debug, Clone, Default)]
pub(crate) struct BrokerSettings {
    pub request_sponge_settings: SpongeSettings,
    pub reduction_sponge_settings: SpongeSettings,
}
