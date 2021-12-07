use crate::{
    brokers::prepare::{
        broker::{Brokerage, Reduction},
        submission::Submission,
        Broker, BrokerFailure, Inclusion, Request,
    },
    data::{PingBoard, Sponge, SpongeSettings},
    discovery::Client,
    view::View,
};

use std::{iter, sync::Arc, time::Duration};

use talk::{crypto::primitives::multi::Signature as MultiSignature, net::SessionConnector};

use zebra::vector::Vector;

impl Broker {
    pub(in crate::brokers::prepare::broker) async fn broker(
        discovery: Arc<Client>,
        view: View,
        connector: Arc<SessionConnector>,
        ping_board: PingBoard,
        brokerages: Vec<Brokerage>,
        reduction_timeout: Option<Duration>,
    ) {
        let mut assignments = Vec::new();
        let mut prepares = Vec::new();
        let mut individual_signatures = Vec::new();

        let mut reduction_inlets = Vec::new();
        let mut outcome_inlets = Vec::new();

        for Brokerage {
            request:
                Request {
                    assignment,
                    prepare,
                    signature,
                },
            reduction_inlet,
            outcome_inlet,
        } in brokerages
        {
            assignments.push(assignment);
            prepares.push(prepare);
            individual_signatures.push(Some(signature));

            reduction_inlets.push(reduction_inlet);
            outcome_inlets.push(outcome_inlet);
        }

        let prepares = Vector::new(prepares).unwrap();
        let inclusions = Inclusion::batch(&prepares);

        let reduction_sponge = Arc::new(Sponge::<(usize, MultiSignature)>::new(SpongeSettings {
            capacity: inclusions.len(),
            timeout: reduction_timeout,
        }));

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

        for (reduction, reduction_inlet) in reductions.into_iter().zip(reduction_inlets) {
            let _ = reduction_inlet.send(Ok(reduction));
        }

        let reductions = reduction_sponge.flush().await;

        let reduction_signature =
            MultiSignature::aggregate(reductions.into_iter().map(|(index, shard)| {
                individual_signatures[index] = None;
                shard
            }))
            .unwrap();

        let submission = Submission::new(
            assignments,
            prepares,
            reduction_signature,
            individual_signatures,
        );

        let outcome = Broker::orchestrate(discovery, view, connector, ping_board, submission)
            .await
            .map_err(|_| BrokerFailure::Error);

        for outcome_inlet in outcome_inlets {
            let _ = outcome_inlet.send(outcome.clone());
        }
    }
}
