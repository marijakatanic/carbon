use crate::{
    account::Id,
    commit::{BatchCompletionShard, BatchCompletionStatement},
    crypto::{Aggregator, Certificate, Identify},
    discovery::Client,
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use std::collections::{BTreeSet, HashMap};

use talk::crypto::{primitives::hash::Hash, KeyCard};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BatchCompletion {
    view: Hash,
    root: Hash,
    exceptions: BTreeSet<Id>,
    certificate: Certificate,
}

pub(crate) struct BatchCompletionAggregator {
    view: View,
    root: Hash,
    aggregators: HashMap<BTreeSet<Id>, Aggregator<BatchCompletionStatement>>,
}

#[derive(Doom)]
pub(crate) enum BatchCompletionError {
    #[doom(description("`View` unknown"))]
    ViewUnknown,
    #[doom(description("`Certificate` invalid"))]
    CertificateInvalid,
}

impl BatchCompletion {
    pub fn root(&self) -> Hash {
        self.root
    }

    pub fn excepts(&self, id: Id) -> bool {
        self.exceptions.contains(&id)
    }

    pub fn validate(&self, discovery: &Client) -> Result<(), Top<BatchCompletionError>> {
        let view = discovery
            .view(&self.view)
            .ok_or(BatchCompletionError::ViewUnknown.into_top())
            .spot(here!())?;

        let statement =
            BatchCompletionStatement::new(self.view, self.root, self.exceptions.clone());

        self.certificate
            .verify_quorum(&view, &statement)
            .pot(BatchCompletionError::CertificateInvalid, here!())?;

        Ok(())
    }
}

impl BatchCompletionAggregator {
    pub fn new(view: View, root: Hash) -> Self {
        BatchCompletionAggregator {
            view,
            root,
            aggregators: HashMap::new(),
        }
    }

    pub fn add(&mut self, completer: &KeyCard, shard: BatchCompletionShard) {
        let statement =
            BatchCompletionStatement::new(self.view.identifier(), self.root, shard.exceptions());

        let aggregator = Aggregator::new(self.view.clone(), statement);

        let aggregator = self
            .aggregators
            .entry(shard.exceptions())
            .or_insert(aggregator);

        // Assuming that `shard` is valid, `shard.signature()` is valid
        aggregator.add(completer, shard.signature()).unwrap();
    }

    pub fn complete(&self) -> bool {
        self.aggregators
            .iter()
            .find(|(_, aggregator)| aggregator.multiplicity() >= self.view.quorum())
            .is_some()
    }

    pub fn finalize(self) -> BatchCompletion {
        let BatchCompletionAggregator {
            view,
            root,
            aggregators,
        } = self;

        // Assuming that `self.complete()`, exactly one `Aggregator` in `aggregators` has reached a quorum multiplicity
        let (exceptions, aggregator) = aggregators
            .into_iter()
            .find(|(_, aggregator)| aggregator.multiplicity() >= view.quorum())
            .unwrap();

        let (_, certificate) = aggregator.finalize();

        BatchCompletion {
            view: view.identifier(),
            root,
            exceptions,
            certificate,
        }
    }
}
