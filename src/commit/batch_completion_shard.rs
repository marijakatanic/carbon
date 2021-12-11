use crate::{
    account::Id,
    commit::{BatchCompletionStatement, Payload},
    crypto::Identify,
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;

use talk::crypto::{
    primitives::{hash::Hash, multi::Signature as MultiSignature},
    KeyCard, KeyChain,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BatchCompletionShard {
    exceptions: BTreeSet<Id>,
    signature: MultiSignature,
}

#[derive(Doom)]
pub(crate) enum BatchCompletionShardError {
    #[doom(description("Foreign exception"))]
    ForeignException,
    #[doom(description("Signature invalid"))]
    SignatureInvalid,
}

impl BatchCompletionShard {
    pub fn new<I>(keychain: &KeyChain, view: Hash, root: Hash, exceptions: I) -> Self
    where
        I: IntoIterator<Item = Id>,
    {
        let exceptions = exceptions.into_iter().collect::<BTreeSet<_>>();

        let statement = BatchCompletionStatement::new(view, root, exceptions.clone());
        let signature = keychain.multisign(&statement).unwrap();

        BatchCompletionShard {
            exceptions,
            signature,
        }
    }

    pub fn exceptions(&self) -> BTreeSet<Id> {
        self.exceptions.clone()
    }

    pub fn signature(&self) -> MultiSignature {
        self.signature.clone()
    }

    pub fn validate(
        &self,
        view: &View,
        root: Hash,
        payloads: &[Payload],
        completer: &KeyCard,
    ) -> Result<(), Top<BatchCompletionShardError>> {
        for id in self.exceptions.iter() {
            payloads
                .binary_search_by_key(id, |payload| payload.id())
                .map_err(|_| BatchCompletionShardError::ForeignException.into_top())
                .spot(here!())?;
        }

        let exceptions = self.exceptions.clone();
        let statement = BatchCompletionStatement::new(view.identifier(), root, exceptions);

        self.signature
            .verify([completer], &statement)
            .pot(BatchCompletionShardError::SignatureInvalid, here!())?;

        Ok(())
    }
}
