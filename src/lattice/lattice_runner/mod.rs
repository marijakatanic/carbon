use crate::{
    discovery::Client,
    lattice::{
        messages::DisclosureSend, Element as LatticeElement, Instance as LatticeInstance, Message,
        MessageError,
    },
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use std::collections::HashMap;
use std::sync::Arc;

use talk::crypto::Identity;
use talk::crypto::{KeyCard, KeyChain};
use talk::sync::fuse::Fuse;
use talk::unicast::{Acknowledgement, Acknowledger, PushSettings, Receiver, Sender};
use talk::{broadcast::BestEffortSettings, crypto::primitives::hash::Hash};

use tokio::sync::oneshot::{Receiver as OneshotReceiver, Sender as OneshotSender};

type ProposalInlet<Element> = OneshotSender<(Element, ResultInlet)>;
type ProposalOutlet<Element> = OneshotReceiver<(Element, ResultInlet)>;

type ResultInlet = OneshotSender<bool>;
type ResultOutlet = OneshotReceiver<bool>;

pub(in crate::lattice) struct LatticeRunner<Instance: LatticeInstance, Element: LatticeElement> {
    view: View,
    instance: Instance,

    members: HashMap<Identity, KeyCard>,

    keychain: KeyChain,
    database: Database<Instance, Element>,

    discovery: Arc<Client>,
    sender: Sender<Message<Instance, Element>>,
    receiver: Receiver<Message<Instance, Element>>,

    proposal_outlet: ProposalOutlet<Element>,

    settings: Settings,
    fuse: Fuse,
}

struct Database<Instance: LatticeInstance, Element: LatticeElement> {
    safe_elements: HashMap<Hash, Element>,
    disclosure: DisclosureDatabase<Instance, Element>,
}

struct DisclosureDatabase<Instance: LatticeInstance, Element: LatticeElement> {
    disclosed: bool,
    disclosures: HashMap<(Identity, Hash), DisclosureSend<Instance, Element>>,
}

struct Settings {
    broadcast: BestEffortSettings,
}

#[derive(Doom)]
enum HandleError {
    #[doom(description("Message from a source foreign to the `View`"))]
    ForeignSource,
    #[doom(description("Invalid message"))]
    InvalidMessage,
}

impl<Instance, Element> LatticeRunner<Instance, Element>
where
    Instance: LatticeInstance,
    Element: LatticeElement,
{
    pub fn new(
        view: View,
        instance: Instance,
        keychain: KeyChain,
        discovery: Arc<Client>,
        sender: Sender<Message<Instance, Element>>,
        receiver: Receiver<Message<Instance, Element>>,
        proposal_outlet: ProposalOutlet<Element>,
    ) -> Self {
        let members = view
            .members()
            .iter()
            .cloned()
            .map(|keycard| (keycard.identity(), keycard))
            .collect();

        let database = Database {
            safe_elements: HashMap::new(),
            disclosure: DisclosureDatabase {
                disclosed: false,
                disclosures: HashMap::new(),
            },
        };

        // TODO: Forward variable settings
        let settings = Settings {
            broadcast: BestEffortSettings {
                push_settings: PushSettings {
                    stop_condition: Acknowledgement::Strong,
                    ..Default::default()
                },
            },
        };

        let fuse = Fuse::new();

        LatticeRunner {
            view,
            instance,
            members,
            keychain,
            database,
            discovery,
            sender,
            receiver,
            proposal_outlet,
            settings,
            fuse,
        }
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                Ok((proposal, result_inlet)) = &mut self.proposal_outlet => {
                    self.handle_proposal(proposal, result_inlet).await;
                }

                (source, message, acknowledger) = self.receiver.receive() => {
                    let _ = self.handle_message(source, message, acknowledger).await;
                }
            }
        }
    }

    async fn handle_proposal(&mut self, proposal: Element, result_inlet: ResultInlet) {
        if !self.disclosed() {
            self.disclose(proposal).await;
            let _ = result_inlet.send(true);
        } else {
            let _ = result_inlet.send(false);
        }
    }

    async fn handle_message(
        &mut self,
        source: Identity,
        message: Message<Instance, Element>,
        acknowledger: Acknowledger,
    ) -> Result<(), Top<HandleError>> {
        if let Some(keycard) = self.members.get(&source).cloned() {
            self.validate_message(&keycard, &message)
                .pot(HandleError::InvalidMessage, here!())?;

            self.process_message(&keycard, message, acknowledger);

            Ok(())
        } else {
            HandleError::ForeignSource.fail().spot(here!())
        }
    }

    fn validate_message(
        &self,
        source: &KeyCard,
        message: &Message<Instance, Element>,
    ) -> Result<(), Top<MessageError>> {
        match message {
            Message::DisclosureSend(message) => self.validate_disclosure_send(source, message),
        }
    }

    fn process_message(
        &mut self,
        source: &KeyCard,
        message: Message<Instance, Element>,
        acknowledger: Acknowledger,
    ) {
        match message {
            Message::DisclosureSend(message) => {
                self.process_disclosure_send(source, message, acknowledger);
            }
        }
    }
}

// Implementations

mod disclosure;
mod message_handlers;
