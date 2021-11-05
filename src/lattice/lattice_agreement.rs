use crate::{
    discovery::Client,
    lattice::{Element as LatticeElement, Instance as LatticeInstance, LatticeRunner, Message},
    view::View,
};

use doomstack::{Doom, Top};

use std::sync::Arc;

use talk::crypto::KeyChain;
use talk::net::{Connector, Listener};
use talk::sync::fuse::Fuse;
use talk::unicast::{Receiver, Sender};

use tokio::sync::oneshot;
use tokio::sync::oneshot::{Receiver as OneshotReceiver, Sender as OneshotSender};

type ProposalInlet<Element> = OneshotSender<(Element, ResultInlet)>;
type ProposalOutlet<Element> = OneshotReceiver<(Element, ResultInlet)>;

type ResultInlet = OneshotSender<bool>;
type ResultOutlet = OneshotReceiver<bool>;

pub(crate) struct LatticeAgreement<Instance: LatticeInstance, Element: LatticeElement> {
    instance: Instance,
    proposal_inlet: Option<ProposalInlet<Element>>,
    _fuse: Fuse,
}

#[derive(Doom)]
pub(crate) enum LatticeAgreementError {
    #[doom(description("Proposal superseded"))]
    ProposalSuperseded,
}

impl<Instance, Element> LatticeAgreement<Instance, Element>
where
    Instance: LatticeInstance,
    Element: LatticeElement,
{
    pub fn new<C, L>(
        view: View,
        instance: Instance,
        keychain: KeyChain,
        discovery: Arc<Client>,
        connector: C,
        listener: L,
    ) -> Self
    where
        C: Connector,
        L: Listener,
    {
        let sender: Sender<Message<Instance, Element>> = Sender::new(connector, Default::default()); // TODO: Forward settings
        let receiver: Receiver<Message<Instance, Element>> =
            Receiver::new(listener, Default::default()); // TODO: Forward settings

        let (proposal_inlet, proposal_outlet) = oneshot::channel();
        let proposal_inlet = Some(proposal_inlet);

        let fuse = Fuse::new();

        {
            let instance = instance.clone();
            let mut runner = LatticeRunner::new(
                view,
                instance,
                keychain,
                discovery,
                sender,
                receiver,
                proposal_outlet,
            );

            fuse.spawn(async move {
                let _ = runner.run().await;
            });
        }

        LatticeAgreement {
            instance,
            proposal_inlet,
            _fuse: fuse,
        }
    }

    async fn propose(&mut self, element: Element) -> Result<(), Top<LatticeAgreementError>> {
        let proposal_inlet = self
            .proposal_inlet
            .take()
            .expect("called `LatticeAgreement::propose` more than once");

        let (result_inlet, result_outlet) = oneshot::channel();

        let _ = proposal_inlet.send((element, result_inlet));

        // This cannot fail as the corresponding `result_inlet` is
        // sent to `run`, which keeps running for as long as
        // `self` exists
        if result_outlet.await.unwrap() {
            Ok(())
        } else {
            LatticeAgreementError::ProposalSuperseded.fail()
        }
    }
}
