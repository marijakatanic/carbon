use crate::{data::SpongeSettings, signup::SignupSettings};


#[derive(Debug, Clone, Default)]
pub(crate) struct BrokerSettings {
    pub signup_settings: SignupSettings,
    pub sponge_settings: SpongeSettings
}