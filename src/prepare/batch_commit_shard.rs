use crate::{
    account::Id,
    crypto::Identify,
    discovery::Client,
    prepare::{BatchCommitStatement, Equivocation, Prepare},
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
            .map(|equivocation| (equivocation.id(), equivocation))
            .collect::<HashMap<_, _>>();

        let statement = BatchCommitStatement::new(view, root, exceptions.keys().copied().collect());
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
        root: Hash,
        prepares: &[Prepare],
        committer: &KeyCard,
    ) -> Result<(), Top<BatchCommitShardError>> {
        for (id, equivocation) in self.exceptions.iter() {
            // Assuming that `prepares` was generated locally, it is is sorted by `Id`,
            // and can therefore be searched using `binary_search*`
            prepares
                .binary_search_by_key(id, |prepare| prepare.id())
                .map_err(|_| BatchCommitShardError::ForeignException.into_top())
                .spot(here!())?;

            if equivocation.id() != *id {
                return BatchCommitShardError::MismatchedId.fail().spot(here!());
            }

            equivocation
                .validate(discovery)
                .pot(BatchCommitShardError::EquivocationInvalid, here!())?;
        }

        let exceptions = self.exceptions.keys().copied().collect();
        let statement = BatchCommitStatement::new(view.identifier(), root, exceptions);

        self.signature
            .verify([committer], &statement)
            .pot(BatchCommitShardError::SignatureInvalid, here!())?;

        Ok(())
    }
}
