use crate::{
    crypto::Identify,
    lattice::{
        message::Message,
        messages::{CertificationConfirmation, CertificationRequest, CertificationUpdate},
        Element as LatticeElement, Instance as LatticeInstance, LatticeRunner, MessageError,
    },
};

use doomstack::{Doom, Top};

use std::collections::BTreeSet;

use talk::crypto::KeyCard;
use talk::unicast::{Acknowledgement, Acknowledger, PushSettings};

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
        if message.decision.view != self.view.identifier() {
            return MessageError::WrongView.fail();
        }

        if message.decision.instance != self.instance {
            return MessageError::WrongInstance.fail();
        }

        if message.decision.elements.is_empty() {
            return MessageError::EmptyDecision.fail();
        }

        if !message.decision.elements.is_subset(&self.database.safe_set) {
            return MessageError::InvalidElement.fail();
        }

        Ok(())
    }

    pub(in crate::lattice::lattice_runner) fn process_certification_request(
        &mut self,
        source: &KeyCard,
        message: CertificationRequest<Instance>,
        acknowledger: Acknowledger,
    ) {
        acknowledger.strong();

        if message
            .decision
            .elements
            .is_superset(&self.database.accepted_set)
        {
            let identifier = message.decision.identifier();
            let signature = self.keychain.multisign(&message.decision).unwrap();

            let message = CertificationConfirmation {
                identifier,
                signature,
            };

            self.sender.spawn_push(
                source.identity(),
                Message::CertificationConfirmation(message),
                PushSettings {
                    stop_condition: Acknowledgement::Weak,
                    ..Default::default()
                }, // TODO: Add settings
                &self.fuse,
            );
        } else {
            let identifier = message.decision.identifier();

            let differences = self
                .database
                .accepted_set
                .difference(&message.decision.elements)
                .cloned()
                .collect::<BTreeSet<_>>();

            let message = CertificationUpdate {
                identifier,
                differences,
            };

            self.sender.spawn_push(
                source.identity(),
                Message::CertificationUpdate(message),
                PushSettings {
                    stop_condition: Acknowledgement::Weak,
                    ..Default::default()
                }, // TODO: Add settings
                &self.fuse,
            );
        }

        self.database.accepted_set = self
            .database
            .accepted_set
            .union(&message.decision.elements)
            .cloned()
            .collect::<BTreeSet<_>>();
    }
}
