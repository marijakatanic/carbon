use crate::lattice::{
    lattice_runner::State, messages::DisclosureSend, Element as LatticeElement,
    Instance as LatticeInstance, LatticeRunner, Message,
};

use talk::broadcast::BestEffort;

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

        self.database.elements.insert(identifier, proposal.clone());

        self.database.safe_set.insert(identifier);
        self.database.proposed_set.insert(identifier);

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

    pub(in crate::lattice::lattice_runner) fn deliver_disclosure(&mut self, proposal: Element) {
        let identifier = proposal.identifier();

        self.database.disclosures += 1;
        self.database.safe_set.insert(identifier);

        if self.state == State::Disclosing {
            if !self.disclosed() {
                self.disclose(proposal);
            }

            self.database.proposed_set.insert(identifier);

            if self.database.disclosures >= self.view.quorum() {
                self.state = State::Proposing;
                self.certify(self.database.proposed_set.clone());
            }
        }
    }
}
