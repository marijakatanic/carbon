use crate::lattice::{
    messages::DisclosureSend, statements::Disclosure, Element as LatticeElement,
    Instance as LatticeInstance, LatticeRunner, Message,
};

use talk::broadcast::BestEffort;
use talk::crypto::primitives::hash;

impl<Instance, Element> LatticeRunner<Instance, Element>
where
    Instance: LatticeInstance,
    Element: LatticeElement,
{
    pub(in crate::lattice::lattice_runner) fn disclosed(&self) -> bool {
        self.database.disclosure.disclosed
    }

    pub(in crate::lattice::lattice_runner) async fn disclose(&mut self, proposal: Element) {
        self.database.disclosure.disclosed = true;

        self.database
            .safe_elements
            .insert(hash::hash(&proposal).unwrap(), proposal.clone());

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
}
