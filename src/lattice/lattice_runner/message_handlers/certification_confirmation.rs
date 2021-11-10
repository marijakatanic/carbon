use crate::lattice::{
    lattice_runner::State, messages::CertificationConfirmation, Element as LatticeElement,
    Instance as LatticeInstance, LatticeRunner, MessageError,
};

use doomstack::{here, Doom, ResultExt, Top};

use talk::crypto::KeyCard;
use talk::unicast::Acknowledger;

impl<Instance, Element> LatticeRunner<Instance, Element>
where
    Instance: LatticeInstance,
    Element: LatticeElement,
{
    pub(in crate::lattice::lattice_runner) fn validate_certification_confirmation(
        &self,
        source: &KeyCard,
        message: &CertificationConfirmation,
    ) -> Result<(), Top<MessageError>> {
        if self.state != State::Proposing {
            return MessageError::WrongState.fail();
        }

        if message.identifier != self.database.certification.as_ref().unwrap().identifier {
            return MessageError::StaleMessage.fail();
        }

        message
            .signature
            .verify(
                [source],
                self.database
                    .certification
                    .as_ref()
                    .unwrap()
                    .aggregator
                    .statement(),
            )
            .pot(MessageError::InvalidSignature, here!())?;

        Ok(())
    }

    pub(in crate::lattice::lattice_runner) fn process_certification_confirmation(
        &mut self,
        source: &KeyCard,
        message: CertificationConfirmation,
        acknowledger: Acknowledger,
    ) {
        acknowledger.strong();

        let certification_database = self.database.certification.as_mut().unwrap();

        certification_database
            .aggregator
            .add(source, message.signature)
            .unwrap();

        // If this is reached, then `self.state == State::Proposing` (as `message` passed validation)
        if certification_database.aggregator.multiplicity() >= self.view.quorum() {
            self.decide();
        }
    }
}
