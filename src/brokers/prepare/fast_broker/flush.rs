use crate::{
    brokers::prepare::{broker_settings::BrokerTaskSettings, submission::Submission, FastBroker},
    data::PingBoard,
    discovery::Client,
    prepare::{Prepare, ReductionStatement},
    signup::IdAssignment,
    view::View,
};

use std::sync::Arc;

use log::{error, info};
use rayon::{
    iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use talk::{
    crypto::{
        primitives::{
            hash, multi::Signature as MultiSignature, sign::Signature as SingleSignature,
        },
        KeyChain,
    },
    net::SessionConnector,
};
use zebra::vector::Vector;

use super::CommitInlet;

impl FastBroker {
    pub(in crate::brokers::prepare::fast_broker) async fn flush(
        batch_size: usize,
        batch_number: usize,
        single_sign_percentage: usize,
        clients: Vec<(KeyChain, IdAssignment)>,
        discovery: Arc<Client>,
        view: View,
        ping_board: PingBoard,
        connector: Arc<SessionConnector>,
        settings: BrokerTaskSettings,
        inlet: CommitInlet,
    ) {
        if clients.len() < batch_number {
            error!(
                "Insufficient number of signups for prepare batches of size {}",
                batch_number
            );
            return;
        }

        let mut clients = clients[0..batch_size].to_vec();
        clients.par_sort_by_key(|(_, id_assignment)| id_assignment.id());

        info!("Pre-computing submissions...");

        let submissions = (0..batch_number)
            .map(|number| FastBroker::prepare(single_sign_percentage, number as u64, &clients));

        info!("All submissions pre-computed!");

        info!("Starting prepare...");

        for (i, submission) in submissions.into_iter().enumerate() {
            let discovery = discovery.clone();
            let view = view.clone();
            let ping_board = ping_board.clone();
            let connector = connector.clone();
            let settings = settings.clone();

            info!("Submitting prepare batch {}", i);
            let commit =
                FastBroker::broker(discovery, view, ping_board, connector, submission, settings)
                    .await;

            inlet.send(commit).unwrap();
        }

        info!("Exiting flush");
    }

    fn prepare(
        single_sign_percentage: usize,
        height: u64,
        clients: &Vec<(KeyChain, IdAssignment)>,
    ) -> Submission {
        let operation = hash::hash(&0).unwrap();
        let num_individual = single_sign_percentage * clients.len() / 100;

        info!("Number of individual signatures: {}", num_individual);

        let assignments = clients
            .iter()
            .map(|(_, id_assignment)| id_assignment)
            .cloned()
            .collect();

        let (prepares, individual_signatures): (Vec<Prepare>, Vec<Option<SingleSignature>>) =
            clients
                .par_iter()
                .enumerate()
                .map(|(num, (keychain, assignment))| {
                    let prepare = Prepare::new(assignment.id(), height, operation);
                    if num < num_individual {
                        let sig = keychain.sign(&prepare).unwrap();
                        (prepare, Some(sig))
                    } else {
                        (prepare, None)
                    }
                })
                .unzip();

        let prepares = Vector::new(prepares).unwrap();
        let root = prepares.root();

        info!("Batch root: {:?}", root);

        let multi_sigs: Vec<MultiSignature> = clients
            .par_iter()
            .enumerate()
            .filter_map(|(i, (keychain, _))| {
                if i >= num_individual {
                    Some(keychain)
                } else {
                    None
                }
            })
            .map(|keychain| keychain.multisign(&ReductionStatement::new(root)).unwrap())
            .collect();

        info!("Number of multisignatures: {}", multi_sigs.len());

        let reduction_signature = MultiSignature::aggregate(multi_sigs).unwrap();

        // Prepare `Submission`

        let submission = Submission::new(
            assignments,
            prepares,
            reduction_signature,
            individual_signatures,
        );

        submission
    }
}
