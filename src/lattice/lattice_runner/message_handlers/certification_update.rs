use crate::lattice::{
    lattice_runner::State, messages::CertificationUpdate, Element as LatticeElement,
    Instance as LatticeInstance, LatticeRunner, MessageError,
};

use doomstack::{Doom, Top};

use std::collections::BTreeSet;

use talk::{crypto::KeyCard, unicast::Acknowledger};

impl<Instance, Element> LatticeRunner<Instance, Element>
where
    Instance: LatticeInstance,
    Element: LatticeElement,
{
    pub(in crate::lattice::lattice_runner) fn validate_certification_update(
        &self,
        _source: &KeyCard,
        message: &CertificationUpdate,
    ) -> Result<(), Top<MessageError>> {
        if self.state != State::Proposing {
            return MessageError::WrongState.fail();
        }

        if message.identifier != self.database.certification.as_ref().unwrap().identifier {
            return MessageError::StaleMessage.fail();
        }

        if !message.differences.is_subset(&self.database.safe_set) {
            return MessageError::UnsafeElement.fail();
        }

        if message.differences.is_empty() {
            return MessageError::EmptyCertificationUpdate.fail();
        }

        if !message.differences.is_disjoint(&self.database.proposed_set) {
            return MessageError::OverlappingCertificationUpdate.fail();
        }

        Ok(())
    }

    pub(in crate::lattice::lattice_runner) fn process_certification_update(
        &mut self,
        _source: &KeyCard,
        message: CertificationUpdate,
        acknowledger: Acknowledger,
    ) {
        acknowledger.strong();

        self.database.proposed_set = self
            .database
            .proposed_set
            .union(&message.differences)
            .cloned()
            .collect::<BTreeSet<_>>();

        self.certify(self.database.proposed_set.clone());
    }
}
