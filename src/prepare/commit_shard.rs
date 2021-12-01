use crate::{
    account::Id,
    crypto::Identify,
    discovery::Client,
    prepare::{Batch, CommitStatement, Extract},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use talk::crypto::{
    primitives::{hash::Hash, multi::Signature as MultiSignature},
    KeyCard, KeyChain,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CommitShard {
    exceptions: HashMap<Id, Extract>,
    signature: MultiSignature,
}

#[derive(Doom)]
pub(crate) enum CommitShardError {
    #[doom(description("Foreign exception"))]
    ForeignException,
    #[doom(description("Mismatched id"))]
    MismatchedId,
    #[doom(description("Extract invalid"))]
    ExtractInvalid,
    #[doom(description("Signature invalid"))]
    SignatureInvalid,
}

impl CommitShard {
    pub fn new<E>(keychain: &KeyChain, view: Hash, root: Hash, exceptions: E) -> Self
    where
        E: IntoIterator<Item = Extract>,
    {
        let exceptions = exceptions
            .into_iter()
            .map(|extract| (extract.id(), extract))
            .collect::<HashMap<_, _>>();

        let statement = CommitStatement::new(view, root, exceptions.keys().copied());
        let signature = keychain.multisign(&statement).unwrap();

        CommitShard {
            exceptions,
            signature,
        }
    }

    pub fn validate(
        &self,
        discovery: &Client,
        view: &View,
        batch: &Batch,
        committer: &KeyCard,
    ) -> Result<(), Top<CommitShardError>> {
        for (id, extract) in self.exceptions.iter() {
            batch
                .prepares()
                .binary_search_by_key(id, |prepare| prepare.id())
                .map_err(|_| CommitShardError::ForeignException.into_top())
                .spot(here!())?;

            if extract.id() != *id {
                return CommitShardError::MismatchedId.fail().spot(here!());
            }

            extract
                .validate(discovery)
                .pot(CommitShardError::ExtractInvalid, here!())?;
        }

        let exceptions = self.exceptions.keys().copied();
        let statement = CommitStatement::new(view.identifier(), batch.root(), exceptions);

        self.signature
            .verify([committer], &statement)
            .pot(CommitShardError::SignatureInvalid, here!())?;

        Ok(())
    }
}
