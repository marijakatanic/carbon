use crate::{
    account::Id,
    commit::{BatchCompletionShard, BatchCompletionStatement},
    crypto::{Aggregator, Certificate, Identify},
    view::View,
};

use std::collections::{BTreeSet, HashMap};

use talk::crypto::{primitives::hash::Hash, KeyCard};

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

impl BatchCompletionAggregator {
    pub fn new(view: View, root: Hash) -> Self {
        BatchCompletionAggregator {
            view,
            root,
            aggregators: HashMap::new(),
        }
    }

    pub fn add(&mut self, completer: &KeyCard, shard: BatchCompletionShard) -> bool {
        let statement =
            BatchCompletionStatement::new(self.view.identifier(), self.root, shard.exceptions());

        let aggregator = Aggregator::new(self.view.clone(), statement);

        let aggregator = self
            .aggregators
            .entry(shard.exceptions())
            .or_insert(aggregator);

        // Assuming that `shard` is valid, `shard.signature()` is valid
        aggregator.add(completer, shard.signature()).unwrap();

        aggregator.multiplicity() >= self.view.quorum()
    }

    pub fn finalize(self) -> BatchCompletion {
        let BatchCompletionAggregator {
            view,
            root,
            aggregators,
        } = self;

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
