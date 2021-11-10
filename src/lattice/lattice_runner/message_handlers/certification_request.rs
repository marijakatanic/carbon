use crate::lattice::{
    lattice_runner::State, messages::CertificationRequest, Element as LatticeElement,
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
    pub(in crate::lattice::lattice_runner) fn validate_certification_request(
        &self,
        _source: &KeyCard,
        message: &CertificationRequest<Instance>,
    ) -> Result<(), Top<MessageError>> {
        if self.state != State::Proposing {
            return MessageError::WrongState.fail();
        }

        if message.decision.view != self.view.identifier() {
            return MessageError::WrongView.fail();
        }

        if message.decision.instance != self.instance {
            return MessageError::WrongInstance.fail();
        }

        if !message.decision.elements.is_subset(&self.database.safe_set) {
            return MessageError::InvalidElement.fail();
        }

        Ok(())
    }

    pub(in crate::lattice::lattice_runner) fn process_certification_request(
        &mut self,
        _source: &KeyCard,
        _message: CertificationRequest<Instance>,
        acknowledger: Acknowledger,
    ) {
        acknowledger.strong();

        todo!();
    }
}
