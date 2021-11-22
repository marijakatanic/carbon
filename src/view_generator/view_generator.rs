use crate::{
    churn::Churn,
    crypto::Identify,
    discovery::Client as DiscoveryClient,
    lattice::{Decision, LatticeAgreement},
    view::{Increment, Install, InstallAggregator, View},
    view_generator::{
        messages::{SummarizationRequest, SummarizationResponse},
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

type DecisionInlet = OneshotSender<Install>;
type DecisionOutlet = OneshotReceiver<Install>;

pub(crate) struct ViewGenerator {
    proposal_inlet: Option<ProposalInlet>,
    decision_outlet: DecisionOutlet,
    _fuse: Fuse,
}

impl ViewGenerator {
    pub fn new<C, L>(
        view: View,
        keychain: KeyChain,
        discovery: Arc<DiscoveryClient>,
        connector: C,
        listener: L,
    ) -> Self
    where
        C: Connector,
        L: Listener,
    {
        let connect_dispatcher = ConnectDispatcher::new(connector);
        let listen_dispatcher = ListenDispatcher::new(listener, Default::default()); // TODO: Add settings

        // Setup view lattice

        let view_lattice_context =
            format!("{:?}::view_generator::view_lattice", view.identifier(),);

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

        // Setup sequence lattice

        let sequence_lattice_context =
            format!("{:?}::view_generator::sequence_lattice", view.identifier(),);

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

        // Setup channels and shared memory

        let (proposal_inlet, proposal_outlet) = oneshot::channel();
        let (decision_inlet, decision_outlet) = oneshot::channel();

        let aggregator_slot = Arc::new(Mutex::new(None));

        // Setup summarization

        let summarization_context =
            format!("{:?}::view_generator::summarization", view.identifier(),);

        let summarization_connector = connect_dispatcher.register(summarization_context.clone());
        let summarization_listener = listen_dispatcher.register(summarization_context);

        let summarization_sender =
            Sender::<Message>::new(summarization_connector, Default::default()); // TODO: Add settings

        let summarization_receiver =
            Receiver::<Message>::new(summarization_listener, Default::default()); // TODO: Add settings

        let fuse = Fuse::new();

        // Spawn agreement task

        {
            let view = view.clone();
            let discovery = discovery.clone();
            let summarization_sender = summarization_sender.clone();
            let aggregator_slot = aggregator_slot.clone();

            fuse.spawn(async move {
                ViewGenerator::agree(
                    view,
                    discovery,
                    view_lattice,
                    sequence_lattice,
                    proposal_outlet,
                    aggregator_slot,
                    summarization_sender,
                )
                .await;
            });
        }

        // Spawn summarization task

        fuse.spawn(async move {
            ViewGenerator::serve(
                view,
                discovery,
                keychain,
                aggregator_slot,
                summarization_sender,
                summarization_receiver,
                decision_inlet,
            )
            .await;
        });

        Self {
            proposal_inlet: Some(proposal_inlet),
            decision_outlet,
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
        (&mut self.decision_outlet).await.unwrap()
    }

    async fn agree(
        view: View,
        discovery: Arc<DiscoveryClient>,
        mut view_lattice: LatticeAgreement<LatticeInstance, ViewLatticeElement>,
        mut sequence_lattice: LatticeAgreement<LatticeInstance, SequenceLatticeElement>,
        proposal_outlet: ProposalOutlet,
        aggregator_slot: Arc<Mutex<Option<InstallAggregator>>>,
        summarization_sender: Sender<Message>,
    ) {
        // Obtain `view_lattice`'s decision
        let (view_lattice_decision, certificate) = tokio::select! {
            Ok(view_lattice_proposal) = proposal_outlet => {
                let _ = view_lattice.propose(view_lattice_proposal).await;
                view_lattice.decide().await
            }

            output = view_lattice.decide() => {
                output
            }
        };

        // Brief all elements of `view_lattice_decision`

        let view_lattice_decision = view_lattice_decision
            .into_iter()
            .map(|element| element.to_brief(&discovery, &view))
            .collect();

        // Build and submit proposal to `sequence_lattice_decision`

        let sequence_lattice_proposal = SequenceLatticeElement {
            view_lattice_decision,
            certificate,
        };

        let _ = sequence_lattice.propose(sequence_lattice_proposal).await;
        let (sequence_lattice_decision, certificate) = sequence_lattice.decide().await;

        // Brief all elements of `sequence_lattice_decision`

        let sequence_lattice_decision = sequence_lattice_decision
            .clone()
            .into_iter()
            .map(SequenceLatticeElement::to_brief)
            .collect::<Vec<_>>();

        // Initialize `aggregator`

        let increments = ViewGenerator::summarize(&discovery, sequence_lattice_decision.clone());
        let aggregator = InstallAggregator::new(view.clone(), increments);
        *aggregator_slot.lock().unwrap() = Some(aggregator);

        // Issue `SummarizationRequest`

        let precursor = InstallPrecursor {
            sequence_lattice_decision,
            certificate,
        };

        let brief = Message::SummarizationRequest(SummarizationRequest::Brief {
            precursor: precursor.identifier(),
        });

        let expanded = Message::SummarizationRequest(SummarizationRequest::Expanded { precursor });

        let broadcast = BestEffort::brief(
            summarization_sender,
            view.members().iter().map(KeyCard::identity),
            brief,
            expanded,
            Default::default(), // TODO: Add settings
        );

        let fuse = Fuse::new();
        broadcast.spawn(&fuse);

        // This maintains lattices and broadcast running until `self._fuse` is dropped along with `self`
        future::pending::<()>().await;
    }

    async fn serve(
        view: View,
        discovery: Arc<DiscoveryClient>,
        keychain: KeyChain,
        aggregator_slot: Arc<Mutex<Option<InstallAggregator>>>,
        summarization_sender: Sender<Message>,
        mut summarization_receiver: Receiver<Message>,
        decision_inlet: DecisionInlet,
    ) {
        let members = view
            .members()
            .iter()
            .map(|member| (member.identity(), member.clone()))
            .collect::<HashMap<_, _>>();

        let mut aggregator: Option<InstallAggregator> = None;
        let mut signature_cache: HashMap<Hash, MultiSignature> = HashMap::new();

        let mut decision_inlet = Some(decision_inlet);

        let fuse = Fuse::new();

        loop {
            let (source, message, acknowledger) = summarization_receiver.receive().await;

            let keycard = match members.get(&source) {
                Some(keycard) => keycard,
                None => continue,
            };

            match message {
                Message::SummarizationRequest(SummarizationRequest::Brief { precursor }) => {
                    // If `precursor` is in `signature_cache`, reply with the appropriate `MultiSignature`,
                    // otherwise, ask `source` to expand
                    if let Some(signature) = signature_cache.get(&precursor).cloned() {
                        acknowledger.strong();

                        let message =
                            Message::SummarizationResponse(SummarizationResponse { signature });

                        summarization_sender.spawn_push(
                            source,
                            message,
                            PushSettings {
                                stop_condition: Acknowledgement::Weak,
                                ..Default::default() // TODO: Add settings
                            },
                            &fuse,
                        );
                    } else {
                        acknowledger.expand();
                    }
                }
                Message::SummarizationRequest(SummarizationRequest::Expanded { precursor }) => {
                    acknowledger.strong();

                    let identifier = precursor.identifier();

                    let InstallPrecursor {
                        sequence_lattice_decision,
                        certificate,
                    } = precursor;

                    // Verify that `sequence_lattice_decision` was produced by `view`'s `sequence_lattice`

                    let decision = Decision::new(
                        view.identifier(),
                        LatticeInstance::SequenceLattice,
                        sequence_lattice_decision.iter(),
                    );

                    if certificate.verify_quorum(&view, &decision).is_err() {
                        continue;
                    }

                    // Summarize `sequence_lattice_decision`, sign and cache the corresponding `Install`

                    let increments =
                        ViewGenerator::summarize(&*discovery, sequence_lattice_decision);

                    let signature = Install::certify(&keychain, &view, increments);

                    signature_cache.insert(identifier, signature);

                    let message =
                        Message::SummarizationResponse(SummarizationResponse { signature });

                    summarization_sender.spawn_push(
                        source,
                        message,
                        PushSettings {
                            stop_condition: Acknowledgement::Weak,
                            ..Default::default() // TODO: Add settings
                        },
                        &fuse,
                    );
                }
                Message::SummarizationResponse(confirm) => {
                    acknowledger.strong();

                    // Try to pull `aggregator` from `aggregator_slot`

                    if aggregator.is_none() {
                        aggregator = aggregator_slot.lock().unwrap().take();
                    }

                    // If `aggregator` is available, add `confirm.signature` to it;
                    // if a plurality is reached, send `install` via `decision_inlet`
                    aggregator = if let Some(mut aggregator) = aggregator.take() {
                        let _ = aggregator.add(keycard, confirm.signature);

                        if aggregator.multiplicity() >= view.plurality() {
                            let install = aggregator.finalize();
                            let _ = decision_inlet.take().unwrap().send(install);

                            None
                        } else {
                            Some(aggregator)
                        }
                    } else {
                        None
                    };
                }
            }
        }
    }

    fn summarize(
        discovery: &DiscoveryClient,
        sequence_lattice_decision: Vec<SequenceLatticeBrief>,
    ) -> Vec<Increment> {
        // The workings of this function are highly non-trivial, and should be altered with
        // caution. For more information, refer to the theoretical body of work concerning
        // asynchronous reconfiguration. Throughout the remainder of this function's comments,
        // we use `view` to identify the value of the variable `view` in the caller function
        // of `summarize`.

        // Transform each element of `sequence_lattice_decision` through `summarize_decision`.
        // Each element of `sequences` is a `Vec` of `Increment`s. Because each `Increment` is
        // to be interpreted as successively applied to `view`, each element of `sequences`
        // uniquely identifies a sequence of views following `view`.
        let sequences = sequence_lattice_decision
            .into_iter()
            .map(|proposal| ViewGenerator::summarize_decision(proposal, discovery))
            .collect::<Vec<_>>();

        // The goal of what follows is to compute the union of all sequences of views
        // identified by `sequences`, and represent that union as a sequence of
        // `Increment`s, to be applied to `view`.

        // To each height differential, `differentials` associates the relevant
        // differential to `view`, as produced by any of the elements of `sequences`.
        // More in detail, let `(h, d)` be any element of `differentials`.
        // Let `S` denote the sequences of views identified by `sequences`. (Each
        // element of `S` begins with `view`.) Some `s` exists in `S`, some `v` exists
        // in `s`, such that:
        //  - `h = |v| - |view|`
        //  - `d = v minus view`
        let mut differentials = BTreeMap::new();
        differentials.insert(0, Increment::new());

        // Fill `differentials` according to its definition above
        for sequence in sequences {
            sequence
                .into_iter()
                .fold(Increment::new(), |mut accumulator, mut increment| {
                    accumulator.append(&mut increment);
                    differentials.insert(accumulator.len(), accumulator.clone());
                    accumulator
                });
        }

        let differentials = differentials.into_iter().collect::<Vec<_>>();

        // The difference between any two consecutive elements of `differentials` is an `Increment`.
        // Applied to `view` in sequence, those differences identify a sequence of views equal to
        // the union of all sequences of views identified by the elements of `sequences`.
        differentials
            .windows(2)
            .map(|window| {
                let (low, high) = (&window[0].1, &window[1].1);
                let increment = high.difference(low).cloned().collect::<BTreeSet<_>>();
                increment
            })
            .collect()
    }

    fn summarize_decision(
        decision: SequenceLatticeBrief,
        client: &DiscoveryClient,
    ) -> Vec<Increment> {
        // Similarly to `summarize`, this function should be altered with caution.

        let mut churns = Vec::new();
        let mut tails = Vec::new();

        // Collect each `ViewLatticeBrief::Churn` decision in `churns`,
        // and the tail of each `ViewLatticeBrief::Tail` decision in `tails`
        for decision in decision.view_lattice_decision {
            match decision {
                ViewLatticeBrief::Churn { churn } => {
                    churns.push(churn);
                }
                ViewLatticeBrief::Tail { install } => {
                    let install = client.install(&install).unwrap();
                    tails.push(install.increments()[1..].to_vec());
                }
            }
        }

        // Remark: `tails` are guaranteed to include each other
        let max_tail = tails
            .into_iter()
            .max_by_key(|tail| tail.len())
            .unwrap_or(Vec::new());

        // If no `ViewLatticeBrief::Churn` decision was present, return `max_tail`.
        // Otherwise, return a single increment containing the union of all
        // `churns` and `tails`.
        if churns.is_empty() {
            max_tail
        } else {
            let union_view = max_tail.into_iter().chain(churns).fold(
                Increment::new(),
                |mut accumulator, mut increment| {
                    accumulator.append(&mut increment);
                    accumulator
                },
            );

            vec![union_view]
        }
    }
}
