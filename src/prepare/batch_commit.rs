use crate::{
    account::Id,
    crypto::{Aggregator, Certificate, Identify},
    discovery::Client,
    prepare::{BatchCommitShard, BatchCommitStatement},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use log::error;
use serde::{Deserialize, Serialize};

use std::collections::{BTreeSet, HashMap};

use talk::crypto::{primitives::hash::Hash, KeyCard};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BatchCommit {
    view: Hash,
    root: Hash,
    patches: Vec<Patch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Patch {
    exceptions: BTreeSet<Id>,
    certificate: Certificate,
}

#[derive(Doom)]
pub(crate) enum BatchCommitError {
    #[doom(description("Unknown view"))]
    UnknownView,
    #[doom(description("Invalid certificate"))]
    InvalidCertificate,
    #[doom(description("Overlapping patches"))]
    OverlappingPatches,
    #[doom(description("Insufficient power"))]
    InsufficientPower,
}

impl BatchCommit {
    pub fn new<S>(view: View, root: Hash, shards: S) -> Self
    where
        S: IntoIterator<Item = (KeyCard, BatchCommitShard)>,
    {
        let mut aggregators: HashMap<BTreeSet<Id>, Aggregator<BatchCommitStatement>> =
            HashMap::new();

        for (committer, shard) in shards {
            let aggregator = aggregators.entry(shard.exceptions()).or_insert_with(|| {
                error!("Shard has {} exceptions!", shard.exceptions().len());
                
                let statement =
                    BatchCommitStatement::new(view.identifier(), root, shard.exceptions());

                Aggregator::new(view.clone(), statement)
            });

            // Assuming that `shards` are valid, `shard.signature()` is valid
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

    pub fn root(&self) -> Hash {
        self.root
    }

    pub fn validate(&self, discovery: &Client) -> Result<(), Top<BatchCommitError>> {
        let view = discovery
            .view(&self.view)
            .ok_or(BatchCommitError::UnknownView.into_top())
            .spot(here!())?;

        for patch in self.patches.iter() {
            let statement =
                BatchCommitStatement::new(view.identifier(), self.root, patch.exceptions.clone());

            // Verify only the validity of `patch.certificate`, regardless of power
            // (`distinct_power` is invoked later to determine if quorum is reached overall)
            patch
                .certificate
                .verify_raw(&view, &statement)
                .pot(BatchCommitError::InvalidCertificate, here!())?;
        }

        let power =
            Certificate::distinct_power(self.patches.iter().map(|patch| &patch.certificate))
                .pot(BatchCommitError::OverlappingPatches, here!())?;

        if power < view.quorum() {
            return BatchCommitError::InsufficientPower.fail().spot(here!());
        }

        Ok(())
    }
}
