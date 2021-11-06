use crate::lattice::{
    messages::{DisclosureEcho, DisclosureSend},
    Element as LatticeElement, Instance as LatticeInstance, LatticeRunner, Message, MessageError,
};

use doomstack::{here, Doom, ResultExt, Top};

use std::collections::hash_map::Entry;

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
        source: &KeyCard,
        message: &DisclosureSend<Instance, Element>,
    ) -> Result<(), Top<MessageError>> {
        if message.disclosure.view != self.view.identifier() {
            return MessageError::ForeignView.fail().spot(here!());
        }

        if message.disclosure.instance != self.instance {
            return MessageError::ForeignInstance.fail().spot(here!());
        }

        message
            .signature
            .verify(source, &message.disclosure)
            .pot(MessageError::IncorrectSignature, here!())?;

        message
            .disclosure
            .element
            .validate(&self.discovery, &self.view)
            .pot(MessageError::InvalidElement, here!())
    }

    pub(in crate::lattice::lattice_runner) fn process_disclosure_send(
        &mut self,
        source: &KeyCard,
        message: DisclosureSend<Instance, Element>,
        acknowledger: Acknowledger,
    ) {
        let source = source.identity();
        let identifier = message.identifier();

        self.database
            .disclosure
            .disclosures_received
            .insert((source, identifier), message.clone());

        match self.database.disclosure.echoes_sent.entry(source) {
            Entry::Vacant(entry) => {
                entry.insert(identifier);

                let broadcast = BestEffort::brief(
                    self.sender.clone(),
                    self.members.keys().cloned(),
                    Message::DisclosureEcho(DisclosureEcho::Brief(identifier)),
                    Message::DisclosureEcho(DisclosureEcho::Expanded(message)),
                    self.settings.broadcast.clone(),
                );

                broadcast.spawn(&self.fuse);
                acknowledger.strong()
            }
            Entry::Occupied(entry) if *entry.get() == identifier => acknowledger.strong(),
            _ => (),
        }
    }
}
