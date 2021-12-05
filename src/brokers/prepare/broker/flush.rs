use crate::{
    brokers::prepare::{broker::Brokerage, Broker, Failure},
    data::Sponge,
};

use std::{sync::Arc, time::Duration};

use talk::sync::fuse::Fuse;

impl Broker {
    pub(in crate::brokers::prepare::broker) async fn flush(
        brokerage_sponge: Arc<Sponge<Brokerage>>,
        reduction_timeout: Option<Duration>,
    ) {
        let fuse = Fuse::new();

        loop {
            let brokerages = Broker::prepare(brokerage_sponge.flush().await);

            if brokerages.is_empty() {
                continue;
            }

            let reduction_timeout = reduction_timeout.clone();

            fuse.spawn(async move {
                Broker::broker(brokerages, reduction_timeout).await;
            });
        }
    }

    fn prepare(mut brokerages: Vec<Brokerage>) -> Vec<Brokerage> {
        // Sort `brokerages` by requestor

        brokerages.sort_by_key(|brokerage| brokerage.request.id());

        // Deduplicate and fail `brokerages` by requestor

        // The following implementation does not use `Vec::dedup_*` because,
        // in order to fail a duplicate `Brokerage`, it needs to consume
        // its `outcome_inlet` (which mutable references don't allow)
        let mut previous = None;

        brokerages
            .into_iter()
            .filter_map(|brokerage| {
                if Some(brokerage.request.id()) == previous {
                    let _ = brokerage.reduction_inlet.send(Err(Failure::Throttle));
                    None
                } else {
                    previous = Some(brokerage.request.id());
                    Some(brokerage)
                }
            })
            .collect()
    }
}
