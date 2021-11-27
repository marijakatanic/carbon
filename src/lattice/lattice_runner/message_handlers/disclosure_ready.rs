use crate::lattice::{
    messages::DisclosureReady, Element as LatticeElement, Instance as LatticeInstance,
    LatticeRunner, Message, MessageError,
};

use doomstack::{here, ResultExt, Top};

use talk::{broadcast::BestEffort, crypto::KeyCard, unicast::Acknowledger};

impl<Instance, Element> LatticeRunner<Instance, Element>
where
    Instance: LatticeInstance,
    Element: LatticeElement,
{
    pub(in crate::lattice::lattice_runner) fn validate_disclosure_ready(
        &self,
        _source: &KeyCard,
        message: &DisclosureReady<Element>,
    ) -> Result<(), Top<MessageError>> {
        match message {
            DisclosureReady::Brief { .. } => Ok(()),
            DisclosureReady::Expanded { proposal, .. } => proposal
                .validate(&self.discovery, &self.view)
                .pot(MessageError::InvalidElement, here!()),
        }
    }

    pub(in crate::lattice::lattice_runner) fn process_disclosure_ready(
        &mut self,
        source: &KeyCard,
        message: DisclosureReady<Element>,
        acknowledger: Acknowledger,
    ) {
        let source = source.identity();

        let (origin, identifier, proposal) = match message {
            DisclosureReady::Brief {
                origin,
                proposal: identifier,
            } => {
                let proposal = match self.database.elements.get(&identifier).cloned() {
                    Some(proposal) => proposal,
                    None => {
                        acknowledger.expand();
                        return;
                    }
                };

                (origin, identifier, proposal)
            }
            DisclosureReady::Expanded { origin, proposal } => {
                let identifier = proposal.identifier();

                self.database.elements.insert(identifier, proposal.clone());

                (origin, identifier, proposal)
            }
        };

        acknowledger.strong();

        if self
            .database
            .disclosure
            .ready_collected
            .insert((source, origin))
        {
            let support = self
                .database
                .disclosure
                .ready_support
                .entry((origin, identifier))
                .or_insert(0);

            *support += 1;
            let support = *support;

            if support >= self.view.plurality()
                && self.database.disclosure.ready_sent.insert(origin)
            {
                let brief = DisclosureReady::Brief {
                    origin,
                    proposal: identifier,
                };

                let expanded = DisclosureReady::Expanded {
                    origin,
                    proposal: proposal.clone(),
                };

                let broadcast = BestEffort::brief(
                    self.sender.clone(),
                    self.view.members().keys().cloned(),
                    Message::DisclosureReady(brief),
                    Message::DisclosureReady(expanded),
                    self.configuration.broadcast.clone(),
                );

                broadcast.spawn(&self.fuse);
            }

            if support >= self.view.quorum() && self.database.disclosure.delivered.insert(origin) {
                self.deliver_disclosure(proposal);
            }
        }
    }
}
