use crate::{
    account::Id,
    crypto::Identify,
    discovery::Client,
    prepare::{BatchCommitStatement, Equivocation, WitnessedBatch},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use std::collections::{BTreeSet, HashMap};

use talk::crypto::{
    primitives::{hash::Hash, multi::Signature as MultiSignature},
    KeyCard, KeyChain,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BatchCommitShard {
    exceptions: HashMap<Id, Equivocation>,
    signature: MultiSignature,
}

#[derive(Doom)]
pub(crate) enum BatchCommitShardError {
    #[doom(description("Foreign exception"))]
    ForeignException,
    #[doom(description("Mismatched id"))]
    MismatchedId,
    #[doom(description("Extract invalid"))]
    EquivocationInvalid,
    #[doom(description("Signature invalid"))]
    SignatureInvalid,
}

impl BatchCommitShard {
    pub fn new<E>(keychain: &KeyChain, view: Hash, root: Hash, exceptions: E) -> Self
    where
        E: IntoIterator<Item = Equivocation>,
    {
        let exceptions = exceptions
            .into_iter()
            .map(|extract| (extract.id(), extract))
            .collect::<HashMap<_, _>>();

        let statement = BatchCommitStatement::new(view, root, exceptions.keys().copied());
        let signature = keychain.multisign(&statement).unwrap();

        BatchCommitShard {
            exceptions,
            signature,
        }
    }

    pub fn exceptions(&self) -> BTreeSet<Id> {
        self.exceptions.keys().copied().collect()
    }

    pub fn signature(&self) -> MultiSignature {
        self.signature.clone()
    }

    pub fn validate(
        &self,
        discovery: &Client,
        view: &View,
        batch: &WitnessedBatch,
        committer: &KeyCard,
    ) -> Result<(), Top<BatchCommitShardError>> {
        for (id, equivocation) in self.exceptions.iter() {
            equivocation
                .validate(discovery)
                .pot(BatchCommitShardError::EquivocationInvalid, here!())?;

            if equivocation.id() != *id {
                return BatchCommitShardError::MismatchedId.fail().spot(here!());
            }

            batch
                .prepares()
                .binary_search_by_key(id, |prepare| prepare.id())
                .map_err(|_| BatchCommitShardError::ForeignException.into_top())
                .spot(here!())?;
        }

        let exceptions = self.exceptions.keys().copied();
        let statement = BatchCommitStatement::new(view.identifier(), batch.root(), exceptions);

        self.signature
            .verify([committer], &statement)
            .pot(BatchCommitShardError::SignatureInvalid, here!())?;

        Ok(())
    }
}
