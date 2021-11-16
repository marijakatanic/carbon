use crate::{
    crypto::Identify,
    discovery::Client,
    lattice::LatticeAgreement,
    view::{Install, View},
    view_generator::{SequenceProposal, ViewProposal},
};

use serde::{Deserialize, Serialize};

use std::sync::Arc;

use talk::{
    crypto::KeyChain,
    link::context::{ConnectDispatcher, ListenDispatcher},
    net::{Connector, Listener},
    sync::fuse::Fuse,
};

use tokio::sync::oneshot;
use tokio::sync::oneshot::{Receiver, Sender};

type ProposalInlet = Sender<ViewProposal>;
type ProposalOutlet = Receiver<ViewProposal>;

type DecisionInlet = Sender<Install>;
type DecisionOutlet = Receiver<Install>;

pub(crate) struct ViewGenerator {
    proposal_inlet: ProposalInlet,
    _fuse: Fuse,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
enum LatticeInstance {
    ViewLattice = 0,
    SequenceLattice = 1,
}

impl ViewGenerator {
    pub fn new<C, L>(
        view: View,
        keychain: KeyChain,
        discovery: Arc<Client>,
        connector: C,
        listener: L,
    ) -> Self
    where
        C: Connector,
        L: Listener,
    {
        let connect_dispatcher = ConnectDispatcher::new(connector);
        let listen_dispatcher = ListenDispatcher::new(listener, Default::default()); // TODO: Add settings

        let (proposal_inlet, proposal_outlet) = oneshot::channel();

        let fuse = Fuse::new();

        fuse.spawn(async move {
            ViewGenerator::run(
                view,
                keychain,
                discovery,
                connect_dispatcher,
                listen_dispatcher,
                proposal_outlet,
            )
            .await;
        });

        Self {
            proposal_inlet,
            _fuse: fuse,
        }
    }

    async fn run(
        view: View,
        keychain: KeyChain,
        discovery: Arc<Client>,
        connect_dispatcher: ConnectDispatcher,
        listen_dispatcher: ListenDispatcher,
        proposal_outlet: ProposalOutlet,
    ) {
        // Create lattice agreement instances

        let view_lattice_context = format!(
            "VIEW GENERATOR -- VIEW LATTICE -- VIEW {:?}",
            view.identifier(),
        );

        let sequence_lattice_context = format!(
            "VIEW GENERATOR -- SEQUENCE LATTICE -- VIEW {:?}",
            view.identifier(),
        );

        let summarization_context = format!(
            "VIEW GENERATOR -- SUMMARIZATION -- VIEW {:?}",
            view.identifier(),
        );

        let view_lattice_connector = connect_dispatcher.register(view_lattice_context.clone());
        let view_lattice_listener = listen_dispatcher.register(view_lattice_context);

        let mut view_lattice = LatticeAgreement::<LatticeInstance, ViewProposal>::new(
            view.clone(),
            LatticeInstance::ViewLattice,
            keychain.clone(),
            discovery.clone(),
            view_lattice_connector,
            view_lattice_listener,
        );

        let sequence_lattice_connector =
            connect_dispatcher.register(sequence_lattice_context.clone());
        let sequence_lattice_listener = listen_dispatcher.register(sequence_lattice_context);

        let mut sequence_lattice = LatticeAgreement::<LatticeInstance, SequenceProposal>::new(
            view.clone(),
            LatticeInstance::SequenceLattice,
            keychain,
            discovery.clone(),
            sequence_lattice_connector,
            sequence_lattice_listener,
        );

        // Wait for a proposal or for the view lattice agreement to finish

        let (view_proposals, proof) = tokio::select! {
            Ok(proposal) = proposal_outlet => {
                let _ = view_lattice.propose(proposal).await;
                view_lattice.decide().await
            }

            output = view_lattice.decide() => {
                output
            }
        };

        let sequence_proposal = SequenceProposal {
            proposal: view_proposals
                .into_iter()
                .map(|proposal| proposal.into_decision(&discovery, &view))
                .collect(),
            proof: proof,
        };

        let _ = sequence_lattice.propose(sequence_proposal).await;
        let (sequence_proposals, proof) = sequence_lattice.decide().await;

        // Summarize
    }
}
