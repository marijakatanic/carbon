use crate::lattice::LatticeAgreementSettings;

use talk::{
    link::context::ListenDispatcherSettings,
    unicast::{PartialPushSettings, ReceiverSettings, SenderSettings},
};

#[derive(Debug, Clone, Default)]
pub(crate) struct ViewGeneratorSettings {
    pub listen_dispatcher_settings: ListenDispatcherSettings,
    pub view_lattice_settings: LatticeAgreementSettings,
    pub sequence_lattice_settings: LatticeAgreementSettings,
    pub summarization_sender_settings: SenderSettings,
    pub summarization_receiver_settings: ReceiverSettings,
    pub push_settings: PartialPushSettings,
}
