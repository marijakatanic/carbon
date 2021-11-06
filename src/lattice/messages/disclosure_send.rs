use crate::{
    discovery::Client,
    lattice::{
        statements::Disclosure, Element as LatticeElement, Instance as LatticeInstance,
        LatticeRunner, MessageError,
    },
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::sign::Signature;
use talk::crypto::KeyCard;
use talk::unicast::Acknowledger;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) struct DisclosureSend<Instance, Element> {
    pub disclosure: Disclosure<Instance, Element>,
    pub signature: Signature,
}

impl<Instance, Element> DisclosureSend<Instance, Element>
where
    Instance: LatticeInstance,
    Element: LatticeElement,
{
    pub fn validate(
        &self,
        instance: &Instance,
        client: &Client,
        view: &View,
        source: &KeyCard,
    ) -> Result<(), Top<MessageError>> {
        if self.disclosure.view != view.identifier() {
            return MessageError::ForeignView.fail().spot(here!());
        }

        if self.disclosure.instance != *instance {
            return MessageError::ForeignInstance.fail().spot(here!());
        }

        self.signature
            .verify(source, &self.disclosure)
            .pot(MessageError::IncorrectSignature, here!())?;

        self.disclosure
            .element
            .validate(client, view)
            .pot(MessageError::InvalidElement, here!())
    }
}

impl<Instance, Element> LatticeRunner<Instance, Element>
where
    Instance: LatticeInstance,
    Element: LatticeElement,
{
    pub(in crate::lattice) fn process_disclosure_send(
        &mut self,
        source: KeyCard,
        payload: DisclosureSend<Instance, Element>,
        acknowledger: Acknowledger,
    ) {
    }
}
