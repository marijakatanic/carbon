use crate::lattice::{
    messages::{DisclosureEcho, DisclosureSend},
    Element as LatticeElement, Instance as LatticeInstance, LatticeRunner, Message, MessageError,
};

use doomstack::{here, Doom, ResultExt, Top};

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
        acknowledger.strong();

        let source = source.identity();
        let identifier = message.identifier();

        if self
            .database
            .disclosure
            .disclosures
            .insert(identifier, message.disclosure.clone())
            .is_none()
        {
            // We might have already been prepared to deliver this disclosure (enough ready support)
            // but were waiting to acquire its concrete value (the expanded version)
            let members = self.members.keys().cloned().collect::<Vec<_>>();

            for member in members.into_iter() {
                self.try_deliver_disclosure(member, identifier);
            }
        };

        if self.database.disclosure.echoes_sent.insert(source) {
            let broadcast = BestEffort::brief(
                self.sender.clone(),
                self.members.keys().cloned(),
                Message::DisclosureEcho(DisclosureEcho::Brief {
                    origin: source,
                    disclosure: identifier,
                }),
                Message::DisclosureEcho(DisclosureEcho::Expanded {
                    origin: source,
                    disclosure: message,
                }),
                self.settings.broadcast.clone(),
            );

            broadcast.spawn(&self.fuse);
        }
    }
}
