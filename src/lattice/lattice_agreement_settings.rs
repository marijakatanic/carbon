use std::sync::Arc;

use talk::{
    time::SleepSchedule,
    unicast::{PushSettings, ReceiverSettings, SenderSettings},
};

#[derive(Debug, Clone, Default)]
pub(crate) struct LatticeAgreementSettings {
    pub sender_settings: SenderSettings,
    pub receiver_settings: ReceiverSettings,
    pub push_settings: PartialPushSettings,
}

#[derive(Debug, Clone)]
pub(crate) struct PartialPushSettings {
    pub retry_schedule: Arc<dyn SleepSchedule>,
}

impl Default for PartialPushSettings {
    fn default() -> Self {
        let push_settings = PushSettings::default();

        PartialPushSettings {
            retry_schedule: push_settings.retry_schedule,
        }
    }
}
