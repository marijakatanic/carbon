use crate::lattice::{
    lattice_runner::State, messages::CertificationUpdate, Element as LatticeElement,
    Instance as LatticeInstance, LatticeRunner, MessageError,
};

use doomstack::{Doom, Top};

use talk::crypto::KeyCard;
use talk::unicast::Acknowledger;

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
        if self.database.certification.as_ref().unwrap().identifier != message.identifier {
            return MessageError::StaleMessage.fail();
        }
        if message
            .differences
            .iter()
            .any(|element| !self.database.safe_set.contains_key(element))
        {
            return MessageError::InvalidElement.fail();
        }
        if message
            .differences
            .iter()
            .all(|element| self.database.proposed_set.contains(element).unwrap())
        {
            return MessageError::NoNewElements.fail();
        }

        Ok(())
    }

    pub(in crate::lattice::lattice_runner) fn process_certification_update(
        &mut self,
        _source: &KeyCard,
        _message: CertificationUpdate,
        _acknowledger: Acknowledger,
    ) {
        todo!();
    }
}
