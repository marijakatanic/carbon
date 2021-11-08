use crate::lattice::{
    messages::DisclosureSend, Element as LatticeElement, Instance as LatticeInstance,
    LatticeRunner, Message,
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
        let identifier = proposal.identifier();

        self.database.disclosure.disclosed = true;

        self.database
            .safe_elements
            .insert(identifier, proposal.clone());

        let brief = DisclosureSend::Brief {
            proposal: identifier,
        };

        let expanded = DisclosureSend::Expanded { proposal };

        let broadcast = BestEffort::brief(
            self.sender.clone(),
            self.members.keys().cloned(),
            Message::DisclosureSend(brief),
            Message::DisclosureSend(expanded),
            self.settings.broadcast.clone(),
        );

        broadcast.spawn(&self.fuse);
    }

    pub(in crate::lattice::lattice_runner) fn deliver_disclosure(
        &mut self,
        origin: Identity,
        _proposal: Element,
    ) {
        println!("Disclosure delivered from {:?}.", origin);
    }
}
