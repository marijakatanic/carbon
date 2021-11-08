use crate::lattice::{
    messages::{DisclosureEcho, DisclosureReady},
    Element as LatticeElement, Instance as LatticeInstance, LatticeRunner, Message, MessageError,
};

use doomstack::Top;

use talk::broadcast::BestEffort;
use talk::crypto::KeyCard;
use talk::unicast::Acknowledger;

impl<Instance, Element> LatticeRunner<Instance, Element>
where
    Instance: LatticeInstance,
    Element: LatticeElement,
{
    pub(in crate::lattice::lattice_runner) fn validate_disclosure_echo(
        &self,
        _source: &KeyCard,
        message: &DisclosureEcho<Instance, Element>,
    ) -> Result<(), Top<MessageError>> {
        match message {
            DisclosureEcho::Brief { .. } => Ok(()),
            DisclosureEcho::Expanded { origin, disclosure } => {
                let origin = self.members[origin].clone();
                self.validate_disclosure_send(&origin, disclosure)
            }
        }
    }

    pub(in crate::lattice::lattice_runner) fn process_disclosure_echo(
        &mut self,
        source: &KeyCard,
        message: DisclosureEcho<Instance, Element>,
        acknowledger: Acknowledger,
    ) {
        let source = source.identity();

        let (origin, identifier) = match message {
            DisclosureEcho::Brief { origin, disclosure } => {
                if !self
                    .database
                    .disclosure
                    .disclosures
                    .contains_key(&disclosure)
                {
                    acknowledger.expand();
                    return;
                }

                (origin, disclosure)
            }
            DisclosureEcho::Expanded { origin, disclosure } => {
                let identifier = disclosure.identifier();

                if self
                    .database
                    .disclosure
                    .disclosures
                    .insert(identifier, disclosure.disclosure)
                    .is_none()
                {
                    // We might have already been prepared to deliver this disclosure (enough ready support)
                    // but were waiting to acquire its concrete value (the expanded version)
                    let members = self.members.keys().cloned().collect::<Vec<_>>();

                    for member in members.into_iter() {
                        self.try_deliver_disclosure(member, identifier);
                    }
                };

                (origin, identifier)
            }
        };

        acknowledger.strong();

        if !self
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

            if support >= self.view.quorum() && !self.database.disclosure.ready_sent.insert(origin)
            {
                let broadcast = BestEffort::new(
                    self.sender.clone(),
                    self.members.keys().cloned(),
                    Message::DisclosureReady(DisclosureReady {
                        origin,
                        disclosure: identifier,
                    }),
                    self.settings.broadcast.clone(),
                );

                broadcast.spawn(&self.fuse);
            }
        }
    }
}
