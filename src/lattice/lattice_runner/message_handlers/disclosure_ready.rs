use crate::lattice::{
    messages::DisclosureReady, Element as LatticeElement, Instance as LatticeInstance,
    LatticeRunner, Message, MessageError,
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
    pub(in crate::lattice::lattice_runner) fn validate_disclosure_ready(
        &self,
        _source: &KeyCard,
        _message: &DisclosureReady,
    ) -> Result<(), Top<MessageError>> {
        Ok(())
    }

    pub(in crate::lattice::lattice_runner) fn process_disclosure_ready(
        &mut self,
        source: &KeyCard,
        message: DisclosureReady,
        acknowledger: Acknowledger,
    ) {
        acknowledger.strong();

        let source = source.identity();
        let origin = message.origin;
        let identifier = message.disclosure;

        if !self
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
                && !self.database.disclosure.ready_sent.insert(origin)
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

        self.try_deliver_disclosure(source, identifier);
    }
}
