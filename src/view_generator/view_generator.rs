use crate::{
    churn::Churn,
    crypto::Identify,
    discovery::Client,
    lattice::{Decisions, LatticeAgreement},
    view::{Increment, Install, InstallAggregator, View},
    view_generator::{
        messages::{SummarizeConfirm, SummarizeSend},
        view_lattice_brief::ViewLatticeBrief,
        InstallPrecursor, LatticeInstance, Message, SequenceLatticeBrief, SequenceLatticeElement,
        ViewLatticeElement,
    },
};

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::future;
use std::sync::{Arc, Mutex};

use talk::broadcast::BestEffort;
use talk::crypto::primitives::hash::Hash;
use talk::crypto::primitives::multi::Signature as MultiSignature;
use talk::crypto::{KeyCard, KeyChain};
use talk::link::context::{ConnectDispatcher, ListenDispatcher};
use talk::net::{Connector, Listener};
use talk::sync::fuse::Fuse;
use talk::unicast::{Acknowledgement, PushSettings, Receiver, Sender};

use tokio::sync::oneshot;
use tokio::sync::oneshot::{Receiver as OneshotReceiver, Sender as OneshotSender};

type ProposalInlet = OneshotSender<ViewLatticeElement>;
type ProposalOutlet = OneshotReceiver<ViewLatticeElement>;

type InstallInlet = OneshotSender<Install>;
type InstallOutlet = OneshotReceiver<Install>;

pub(crate) struct ViewGenerator {
    proposal_inlet: Option<ProposalInlet>,
    install_outlet: Option<InstallOutlet>,
    _fuse: Fuse,
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

        let view_lattice_context = format!(
            "VIEW GENERATOR -- VIEW LATTICE -- VIEW {:?}",
            view.identifier(),
        );

        let view_lattice_connector = connect_dispatcher.register(view_lattice_context.clone());
        let view_lattice_listener = listen_dispatcher.register(view_lattice_context);

        let view_lattice = LatticeAgreement::<LatticeInstance, ViewLatticeElement>::new(
            view.clone(),
            LatticeInstance::ViewLattice,
            keychain.clone(),
            discovery.clone(),
            view_lattice_connector,
            view_lattice_listener,
        );

        let sequence_lattice_context = format!(
            "VIEW GENERATOR -- SEQUENCE LATTICE -- VIEW {:?}",
            view.identifier(),
        );

        let sequence_lattice_connector =
            connect_dispatcher.register(sequence_lattice_context.clone());

        let sequence_lattice_listener = listen_dispatcher.register(sequence_lattice_context);

        let sequence_lattice = LatticeAgreement::<LatticeInstance, SequenceLatticeElement>::new(
            view.clone(),
            LatticeInstance::SequenceLattice,
            keychain.clone(),
            discovery.clone(),
            sequence_lattice_connector,
            sequence_lattice_listener,
        );

        let (proposal_inlet, proposal_outlet) = oneshot::channel();

        let summarization_context = format!(
            "VIEW GENERATOR -- SUMMARIZATION -- VIEW {:?}",
            view.identifier(),
        );

        let summarization_connector = connect_dispatcher.register(summarization_context.clone());
        let summarization_listener = listen_dispatcher.register(summarization_context);

        let summarization_sender =
            Sender::<Message>::new(summarization_connector, Default::default());

        let summarization_receiver =
            Receiver::<Message>::new(summarization_listener, Default::default());

        let fuse = Fuse::new();

        let aggregator = Arc::new(Mutex::new(None));

        {
            let view = view.clone();
            let discovery = discovery.clone();
            let summarization_sender = summarization_sender.clone();
            let aggregator = aggregator.clone();

            fuse.spawn(async move {
                ViewGenerator::agree(
                    view,
                    discovery,
                    aggregator,
                    view_lattice,
                    sequence_lattice,
                    proposal_outlet,
                    summarization_sender,
                )
                .await;
            });
        }

        let (install_inlet, install_outlet) = oneshot::channel();

        fuse.spawn(async move {
            ViewGenerator::serve(
                view,
                discovery,
                keychain,
                aggregator,
                summarization_sender,
                summarization_receiver,
                install_inlet,
            )
            .await;
        });

        Self {
            proposal_inlet: Some(proposal_inlet),
            install_outlet: Some(install_outlet),
            _fuse: fuse,
        }
    }

    pub fn propose_churn<C>(&mut self, install: Hash, churn: C)
    where
        C: IntoIterator<Item = Churn>,
    {
        let churn = churn.into_iter().collect();
        let proposal = ViewLatticeElement::Churn { install, churn };

        let _ = self.proposal_inlet.take().unwrap().send(proposal);
    }

    pub fn propose_tail(&mut self, install: Hash) {
        let proposal = ViewLatticeElement::Tail { install };
        let _ = self.proposal_inlet.take().unwrap().send(proposal);
    }

    pub async fn decide(&mut self) -> Install {
        self.install_outlet.take().unwrap().await.unwrap()
    }

    async fn agree(
        view: View,
        discovery: Arc<Client>,
        aggregator: Arc<Mutex<Option<InstallAggregator>>>,
        mut view_lattice: LatticeAgreement<LatticeInstance, ViewLatticeElement>,
        mut sequence_lattice: LatticeAgreement<LatticeInstance, SequenceLatticeElement>,
        proposal_outlet: ProposalOutlet,
        summarization_sender: Sender<Message>,
    ) {
        // Create lattice agreement instances
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

        let sequence_proposal = SequenceLatticeElement {
            view_lattice_decisions: view_proposals
                .into_iter()
                .map(|proposal| proposal.to_brief(&discovery, &view))
                .collect(),
            certificate: proof,
        };

        let _ = sequence_lattice.propose(sequence_proposal).await;
        let (sequence_proposals, certificate) = sequence_lattice.decide().await;

        // Set my proposal

        let increments = ViewGenerator::summarize(
            &discovery,
            sequence_proposals
                .clone()
                .into_iter()
                .map(SequenceLatticeElement::to_brief)
                .collect(),
        );
        *aggregator.lock().unwrap() = Some(InstallAggregator::new(view.clone(), increments));

        // Summarize

        let precursor = InstallPrecursor {
            sequence_lattice_decisions: sequence_proposals
                .into_iter()
                .map(SequenceLatticeElement::to_brief)
                .collect(),
            certificate,
        };

        let brief = Message::SummarizeSend(SummarizeSend::Brief {
            precursor: precursor.identifier(),
        });

        let expanded = Message::SummarizeSend(SummarizeSend::Expanded { precursor });

        let broadcast = BestEffort::brief(
            summarization_sender,
            view.members().iter().map(KeyCard::identity),
            brief,
            expanded,
            Default::default(),
        );

        let fuse = Fuse::new();
        broadcast.spawn(&fuse);

        // This will exit once ViewGenerator is dropped (its fuse is also dropped)
        future::pending::<()>().await;
    }

    async fn serve(
        view: View,
        discovery: Arc<Client>,
        keychain: KeyChain,
        aggregator: Arc<Mutex<Option<InstallAggregator>>>,
        summarization_sender: Sender<Message>,
        mut summarization_receiver: Receiver<Message>,
        install_inlet: InstallInlet,
    ) {
        let fuse = Fuse::new();

        let mut cache: HashMap<Hash, MultiSignature> = HashMap::new();

        let mut local_aggregator: Option<InstallAggregator> = None;

        let members = view
            .members()
            .iter()
            .map(|member| (member.identity(), member.clone()))
            .collect::<HashMap<_, _>>();

        let mut install_inlet = Some(install_inlet);

        loop {
            let (source, message, acknowledger) = summarization_receiver.receive().await;

            let keycard = match members.get(&source) {
                Some(keycard) => keycard,
                None => continue,
            };

            match message {
                Message::SummarizeSend(SummarizeSend::Brief { precursor }) => {
                    if let Some(signature) = cache.get(&precursor).cloned() {
                        acknowledger.strong();

                        let message = Message::SummarizeConfirm(SummarizeConfirm { signature });

                        summarization_sender.spawn_push(
                            source,
                            message,
                            PushSettings {
                                stop_condition: Acknowledgement::Weak,
                                ..Default::default()
                            },
                            &fuse,
                        );
                    } else {
                        acknowledger.expand();
                    }
                }
                Message::SummarizeSend(SummarizeSend::Expanded { precursor }) => {
                    acknowledger.strong();

                    let identifier = precursor.identifier();

                    let InstallPrecursor {
                        sequence_lattice_decisions: decisions,
                        certificate,
                    } = precursor;

                    {
                        // Check this
                        let decisions = Decisions {
                            view: view.identifier(),
                            instance: LatticeInstance::SequenceLattice,
                            elements: decisions.iter().map(Identify::identifier).collect(),
                        };

                        if certificate.verify_quorum(&view, &decisions).is_err() {
                            continue;
                        }
                    }

                    let increments = ViewGenerator::summarize(&*discovery, decisions);

                    let signature = Install::certify(&keychain, &view, increments);

                    cache.insert(identifier, signature);

                    let message = Message::SummarizeConfirm(SummarizeConfirm { signature });

                    summarization_sender.spawn_push(
                        source,
                        message,
                        PushSettings {
                            stop_condition: Acknowledgement::Weak,
                            ..Default::default()
                        },
                        &fuse,
                    );
                }
                Message::SummarizeConfirm(confirm) => {
                    acknowledger.strong();

                    if local_aggregator.is_none() {
                        local_aggregator = aggregator.lock().unwrap().take();
                    }

                    if let Some(mut aggregator) = local_aggregator.take() {
                        let _ = aggregator.add(keycard, confirm.signature);

                        if aggregator.multiplicity() >= view.plurality() {
                            let install = aggregator.finalize();
                            let _ = install_inlet.take().unwrap().send(install);
                        } else {
                            local_aggregator = Some(aggregator);
                        }
                    }
                }
            }
        }
    }

    fn summarize(client: &Client, decisions: Vec<SequenceLatticeBrief>) -> Vec<Increment> {
        let sequences = decisions
            .into_iter()
            .map(|proposal| ViewGenerator::summarize_proposal(proposal, client))
            .collect::<Vec<_>>();

        let mut map = BTreeMap::new();

        for sequence in sequences {
            sequence
                .into_iter()
                .fold(Increment::new(), |mut acc, mut increment| {
                    acc.append(&mut increment);
                    map.insert(acc.len(), acc.clone());
                    acc
                });
        }

        let head = map.iter().next().unwrap().1.clone();

        vec![head]
            .into_iter()
            .chain(
                map.into_iter()
                    .collect::<Vec<_>>()
                    .windows(2)
                    .map(|window| {
                        let (low, high) = (&window[0].1, &window[1].1);
                        let increment = high.difference(low).cloned().collect::<BTreeSet<_>>();
                        increment
                    }),
            )
            .collect::<Vec<_>>()
    }

    fn summarize_proposal(proposal: SequenceLatticeBrief, client: &Client) -> Vec<Increment> {
        let mut tails = Vec::new();
        let mut views = Vec::new();

        for decision in proposal.view_lattice_decisions {
            match decision {
                ViewLatticeBrief::Churn { churn } => {
                    views.push(churn);
                }
                ViewLatticeBrief::Tail { install } => {
                    let install = client.install(&install).unwrap();
                    tails.push(install.increments()[1..].to_owned());
                }
            }
        }

        let greatest_tail = tails
            .into_iter()
            .max_by_key(|tail| tail.len())
            .unwrap_or(Vec::new());

        if views.is_empty() {
            greatest_tail
        } else {
            let last_view = greatest_tail.into_iter().chain(views).fold(
                Increment::new(),
                |mut acc, mut increment| {
                    acc.append(&mut increment);
                    acc
                },
            );

            vec![last_view]
        }
    }
}
