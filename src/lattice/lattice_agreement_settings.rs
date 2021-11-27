use talk::unicast::{PartialPushSettings, ReceiverSettings, SenderSettings};

#[derive(Debug, Clone, Default)]
pub(crate) struct LatticeAgreementSettings {
    pub sender_settings: SenderSettings,
    pub receiver_settings: ReceiverSettings,
    pub push_settings: PartialPushSettings,
}
