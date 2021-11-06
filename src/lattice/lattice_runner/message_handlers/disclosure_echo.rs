use crate::lattice::{
    messages::DisclosureEcho, Element as LatticeElement, Instance as LatticeInstance,
    LatticeRunner, MessageError,
};

use doomstack::Top;

use std::collections::hash_map::Entry;

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

        let identifier = match message {
            DisclosureEcho::Brief { origin, disclosure } => {
                if !self
                    .database
                    .disclosure
                    .disclosures_received
                    .contains_key(&(origin, disclosure))
                {
                    acknowledger.expand();
                    return;
                }

                disclosure
            }
            DisclosureEcho::Expanded { origin, disclosure } => {
                let identifier = disclosure.identifier();

                match self
                    .database
                    .disclosure
                    .disclosures_received
                    .entry((origin, identifier))
                {
                    Entry::Occupied(_) => (),
                    Entry::Vacant(entry) => {
                        entry.insert(disclosure);
                    }
                }

                identifier
            }
        };
    }
}
