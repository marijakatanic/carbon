use crate::lattice::LatticeAgreementSettings;

use talk::link::context::ListenDispatcherSettings;

#[derive(Debug, Clone, Default)]
pub(crate) struct ViewGeneratorSettings {
    pub listen_dispatcher_settings: ListenDispatcherSettings,
    pub view_lattice_settings: LatticeAgreementSettings,
    pub sequence_lattice_settings: LatticeAgreementSettings,
}
