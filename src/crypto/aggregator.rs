use crate::{crypto::Certificate, view::View};

use doomstack::Top;

use talk::crypto::primitives::multi::{MultiError, Signature as MultiSignature};
use talk::crypto::{Identity, KeyCard, Statement};

pub(crate) struct Aggregator<S: Statement> {
    view: View,
    statement: S,
    components: Vec<(Identity, MultiSignature)>,
}

impl<S> Aggregator<S>
where
    S: Statement,
{
    pub fn new(view: View, statement: S) -> Self {
        Aggregator {
            view,
            statement,
            components: Vec::new(),
        }
    }

    pub fn add(
        &mut self,
        keycard: &KeyCard,
        signature: MultiSignature,
    ) -> Result<(), Top<MultiError>> {
        let identity = keycard.identity();

        #[cfg(debug_assertions)]
        {
            if self
                .view
                .members()
                .binary_search_by_key(&identity, |member| member.identity())
                .is_err()
            {
                panic!("Called `Aggregator::add` with foreign `identity`");
            }
        }

        signature.verify([keycard], &self.statement)?;
        self.components.push((identity, signature));

        Ok(())
    }

    pub fn finalize(self) -> Certificate {
        Certificate::aggregate(&self.view, self.components)
    }
}
