use crate::{
    crypto::Aggregator,
    lattice::{
        lattice_runner::{CertificationDatabase, Decision, State},
        messages::CertificationRequest,
        Element as LatticeElement, Instance as LatticeInstance, LatticeRunner, Message,
    },
};

use talk::broadcast::BestEffort;
use talk::sync::fuse::Fuse;

impl<Instance, Element> LatticeRunner<Instance, Element>
where
    Instance: LatticeInstance,
    Element: LatticeElement,
{
    pub(in crate::lattice::lattice_runner) fn current_decision(&self) -> Decision<Instance> {
        if self.state == State::Proposing {
            self.database
                .certification
                .as_ref()
                .unwrap()
                .aggregator
                .statement()
                .clone()
        } else {
            // TODO: Improve this by implementing a collect/iterator method for `zebra::map::Set`

            let mut elements = self
                .database
                .safe_set
                .keys()
                .cloned()
                .filter(|hash| self.database.proposed_set.contains(hash))
                .collect::<Vec<_>>();

            elements.sort();

            Decision {
                elements: elements,
                view: self.view.identifier(),
                instance: self.instance.clone(),
            }
        }
    }

    pub(in crate::lattice::lattice_runner) fn certify(&mut self) {
        let decision = self.current_decision();
        let identifier = decision.identifier();

        let aggregator = Aggregator::new(self.view.clone(), decision.clone());

        let message = CertificationRequest { decision };

        let broadcast = BestEffort::new(
            self.sender.clone(),
            self.members.keys().cloned(),
            Message::CertificationRequest(message),
            self.settings.broadcast.clone(),
        );

        let fuse = Fuse::new();

        broadcast.spawn(&fuse);

        let certification_database = CertificationDatabase {
            identifier,
            aggregator,
            fuse,
        };

        self.database.certification = Some(certification_database);
    }

    pub(in crate::lattice::lattice_runner) fn decide(&mut self) {
        self.state = State::Decided;

        let (_decision, _certificate) = self
            .database
            .certification
            .take()
            .unwrap()
            .aggregator
            .finalize_quorum();

        todo!();
    }
}
