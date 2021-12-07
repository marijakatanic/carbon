use crate::{
    brokers::prepare::{
        broker::{Brokerage, Reduction},
        broker_settings::BrokerTaskSettings,
        Broker, BrokerFailure, Inclusion, Submission, UnzippedBrokerages,
    },
    data::{PingBoard, Sponge, SpongeSettings},
    discovery::Client,
    view::View,
};

use std::{iter, sync::Arc};

use talk::{crypto::primitives::multi::Signature as MultiSignature, net::SessionConnector};

use zebra::vector::Vector;

impl Broker {
    pub(in crate::brokers::prepare::broker) async fn broker(
        discovery: Arc<Client>,
        view: View,
        ping_board: PingBoard,
        connector: Arc<SessionConnector>,
        brokerages: Vec<Brokerage>,
        settings: BrokerTaskSettings,
    ) {
        // Unzip `brokerages` into its components

        let UnzippedBrokerages {
            assignments,
            prepares,
            signatures,
            reduction_inlets,
            commit_inlets,
        } = Brokerage::unzip(brokerages);

        // Initialize `Vec<Option<_>>` of individual signatures

        let mut individual_signatures = signatures
            .into_iter()
            .map(|signature| Some(signature))
            .collect::<Vec<_>>();

        // Wrap `prepares` into a `Vector`, generate an `Inclusion`
        // for each element of `prepares`

        let prepares = Vector::new(prepares).unwrap();
        let inclusions = Inclusion::batch(&prepares);

        // Initialize reduction sponge

        // The capacity of `reduction_sponge` is expressed by `settings.reduction_threshold`
        // as a fraction of `inclusions.len()`: `reduction_sponge` flushes as soon as
        // a `settings.reduction_threshold`-th of the reduction shards are collected.
        let reduction_sponge = Arc::new(Sponge::new(SpongeSettings {
            capacity: ((inclusions.len() as f64) * settings.reduction_threshold) as usize,
            timeout: settings.reduction_timeout,
        }));

        // Build vector of `Reduction`s

        let reductions = inclusions
            .into_iter()
            .zip(iter::repeat(reduction_sponge.clone()))
            .enumerate()
            .map(|(index, (inclusion, reduction_sponge))| Reduction {
                index,
                inclusion,
                reduction_sponge,
            })
            .collect::<Vec<_>>();

        // Send each element of `reductions` to the appropriate `serve` task

        for (reduction, reduction_inlet) in reductions.into_iter().zip(reduction_inlets) {
            let _ = reduction_inlet.send(Ok(reduction));
        }

        // Wait for `reduction_sponge` to flush

        let reduction_shards = reduction_sponge.flush().await;

        // Aggregate reduction signature

        // Each element of `reduction_shards` has been previously verified, and can be
        // aggregated without any further checks
        let reduction_signature =
            MultiSignature::aggregate(reduction_shards.into_iter().map(|(index, shard)| {
                individual_signatures[index] = None;
                shard
            }))
            .unwrap();

        // Prepare `Submission`

        let submission = Submission::new(
            assignments,
            prepares,
            reduction_signature,
            individual_signatures,
        );

        // Orchestrate submission of `submission`

        let commit =
            Broker::orchestrate(discovery, view, connector, ping_board, submission, settings)
                .await
                .map_err(|_| BrokerFailure::Error);

        // Send a copy of `commit` to each `serve` task (note that `commit` is
        // a `Result<BatchCommit, Failure>`)

        for commit_inlet in commit_inlets {
            let _ = commit_inlet.send(commit.clone());
        }
    }
}
