use crate::lattice::{
    messages::{DisclosureEcho, DisclosureReady},
    Element as LatticeElement, Instance as LatticeInstance, LatticeRunner, Message, MessageError,
};

use doomstack::{here, ResultExt, Top};

use talk::{broadcast::BestEffort, crypto::KeyCard, unicast::Acknowledger};

impl<Instance, Element> LatticeRunner<Instance, Element>
where
    Instance: LatticeInstance,
    Element: LatticeElement,
{
    pub(in crate::lattice::lattice_runner) fn validate_disclosure_echo(
        &self,
        _source: &KeyCard,
        message: &DisclosureEcho<Element>,
    ) -> Result<(), Top<MessageError>> {
        match message {
            DisclosureEcho::Brief { .. } => Ok(()),
            DisclosureEcho::Expanded { proposal, .. } => proposal
                .validate(&self.discovery, &self.view)
                .pot(MessageError::InvalidElement, here!()),
        }
    }

    pub(in crate::lattice::lattice_runner) fn process_disclosure_echo(
        &mut self,
        source: &KeyCard,
        message: DisclosureEcho<Element>,
        acknowledger: Acknowledger,
    ) {
        let source = source.identity();

        let (origin, identifier, proposal) = match message {
            DisclosureEcho::Brief {
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
            DisclosureEcho::Expanded { origin, proposal } => {
                let identifier = proposal.identifier();

                self.database.elements.insert(identifier, proposal.clone());

                (origin, identifier, proposal)
            }
        };

        acknowledger.strong();

        if self
            .database
            .disclosure
            .echoes_collected
            .insert((source, origin))
        {
            let support = self
                .database
                .disclosure
                .echo_support
                .entry((origin, identifier))
                .or_insert(0);

            *support += 1;
            let support = *support;

            if support >= self.view.quorum() && self.database.disclosure.ready_sent.insert(origin) {
                let brief = DisclosureReady::Brief {
                    origin,
                    proposal: identifier,
                };

                let expanded = DisclosureReady::Expanded { origin, proposal };

                let broadcast = BestEffort::brief(
                    self.sender.clone(),
                    self.view.members().keys().cloned(),
                    Message::DisclosureReady(brief),
                    Message::DisclosureReady(expanded),
                    self.settings.broadcast.clone(),
                );

                broadcast.spawn(&self.fuse);
            }
        }
    }
}
