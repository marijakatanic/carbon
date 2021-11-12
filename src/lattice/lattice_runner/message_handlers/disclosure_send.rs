use crate::lattice::{
    messages::{DisclosureEcho, DisclosureSend},
    Element as LatticeElement, Instance as LatticeInstance, LatticeRunner, Message, MessageError,
};

use doomstack::{here, ResultExt, Top};

use talk::broadcast::BestEffort;
use talk::crypto::KeyCard;
use talk::unicast::Acknowledger;

impl<Instance, Element> LatticeRunner<Instance, Element>
where
    Instance: LatticeInstance,
    Element: LatticeElement,
{
    pub(in crate::lattice::lattice_runner) fn validate_disclosure_send(
        &self,
        _source: &KeyCard,
        message: &DisclosureSend<Element>,
    ) -> Result<(), Top<MessageError>> {
        match message {
            DisclosureSend::Brief { .. } => Ok(()),
            DisclosureSend::Expanded { proposal } => proposal
                .validate(&self.discovery, &self.view)
                .pot(MessageError::InvalidElement, here!()),
        }
    }

    pub(in crate::lattice::lattice_runner) fn process_disclosure_send(
        &mut self,
        source: &KeyCard,
        message: DisclosureSend<Element>,
        acknowledger: Acknowledger,
    ) {
        let source = source.identity();

        let (identifier, proposal) = match message {
            DisclosureSend::Brief {
                proposal: identifier,
            } => {
                let proposal = match self.database.elements.get(&identifier).cloned() {
                    Some(proposal) => proposal,
                    None => {
                        acknowledger.expand();
                        return;
                    }
                };

                (identifier, proposal)
            }
            DisclosureSend::Expanded { proposal } => {
                let identifier = proposal.identifier();

                self.database.elements.insert(identifier, proposal.clone());

                (identifier, proposal)
            }
        };

        acknowledger.strong();

        if self.database.disclosure.echoes_sent.insert(source) {
            let brief = DisclosureEcho::Brief {
                origin: source,
                proposal: identifier,
            };

            let expanded = DisclosureEcho::Expanded {
                origin: source,
                proposal,
            };

            let broadcast = BestEffort::brief(
                self.sender.clone(),
                self.members.keys().cloned(),
                Message::DisclosureEcho(brief),
                Message::DisclosureEcho(expanded),
                self.settings.broadcast.clone(),
            );

            broadcast.spawn(&self.fuse);
        }
    }
}
