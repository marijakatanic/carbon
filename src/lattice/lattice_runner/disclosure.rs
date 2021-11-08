use crate::lattice::{
    messages::DisclosureSend, statements::Disclosure, Element as LatticeElement,
    Instance as LatticeInstance, LatticeRunner, Message,
};

use talk::broadcast::BestEffort;
use talk::crypto::Identity;

impl<Instance, Element> LatticeRunner<Instance, Element>
where
    Instance: LatticeInstance,
    Element: LatticeElement,
{
    pub(in crate::lattice::lattice_runner) fn disclosed(&self) -> bool {
        self.database.disclosure.disclosed
    }

    pub(in crate::lattice::lattice_runner) fn disclose(&mut self, proposal: Element) {
        self.database.disclosure.disclosed = true;

        self.database
            .safe_elements
            .insert(proposal.identifier(), proposal.clone());

        let disclosure = Disclosure {
            view: self.view.identifier(),
            instance: self.instance.clone(),
            element: proposal,
        };

        let signature = self.keychain.sign(&disclosure).unwrap();

        let disclosure_send = DisclosureSend {
            disclosure,
            signature,
        };

        let message = Message::DisclosureSend(disclosure_send);

        let broadcast = BestEffort::new(
            self.sender.clone(),
            self.members.keys().cloned(),
            message,
            self.settings.broadcast.clone(),
        );

        broadcast.spawn(&self.fuse);
    }

    pub(in crate::lattice::lattice_runner) fn deliver_disclosure(
        &mut self,
        origin: Identity,
        proposal: Element,
    ) {
        // TODO: Implements rest of Lattice Agreement
    }
}
