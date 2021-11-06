use crate::lattice::{
    messages::DisclosureSend, Element as LatticeElement, Instance as LatticeInstance,
    LatticeRunner, MessageError,
};

use doomstack::{here, Doom, ResultExt, Top};

use talk::{crypto::KeyCard, unicast::Acknowledger};

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
        &self,
        _source: &KeyCard,
        _message: DisclosureSend<Instance, Element>,
        _acknowledger: Acknowledger,
    ) {
        todo!()
    }
}
