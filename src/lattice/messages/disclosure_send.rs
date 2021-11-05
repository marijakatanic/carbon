use crate::{
    discovery::Client,
    lattice::{statements::Disclosure, Element as LatticeElement, Instance as LatticeInstance},
    view::View,
};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::sign::Signature;
use talk::crypto::KeyCard;

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
    ) -> bool {
        // Wrong view or instance
        if self.disclosure.view != view.identifier() || self.disclosure.instance != *instance {
            return false;
        }

        // Incorrectly signed
        if self.signature.verify(source, &self.disclosure).is_err() {
            return false;
        }

        // Invalid element
        self.disclosure.element.validate(client, view).is_ok()
    }
}
