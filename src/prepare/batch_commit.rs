use crate::{
    account::Id,
    crypto::{Aggregator, Certificate, Identify},
    prepare::{BatchCommitShard, BatchCommitStatement},
    view::View,
};

use std::collections::{BTreeSet, HashMap};

use talk::crypto::{primitives::hash::Hash, KeyCard};

pub(crate) struct BatchCommit {
    view: Hash,
    root: Hash,
    patches: Vec<Patch>,
}

struct Patch {
    exceptions: BTreeSet<Id>,
    certificate: Certificate,
}

impl BatchCommit {
    pub fn new<S>(view: View, root: Hash, shards: S) -> Self
    where
        S: IntoIterator<Item = (KeyCard, BatchCommitShard)>,
    {
        let mut aggregators: HashMap<BTreeSet<Id>, Aggregator<BatchCommitStatement>> =
            HashMap::new();

        for (committer, shard) in shards {
            let exceptions = shard.exceptions();

            let view = view.clone();
            let statement = BatchCommitStatement::new(view.identifier(), root, exceptions.clone());

            let aggregator = aggregators
                .entry(exceptions)
                .or_insert(Aggregator::new(view, statement));

            aggregator.add(&committer, shard.signature()).unwrap();
        }

        let view = view.identifier();

        let patches = aggregators
            .into_iter()
            .map(|(exceptions, aggregator)| {
                let (_, certificate) = aggregator.finalize();

                Patch {
                    exceptions,
                    certificate,
                }
            })
            .collect();

        BatchCommit {
            view,
            root,
            patches,
        }
    }
}
