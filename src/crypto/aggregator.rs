use crate::{crypto::Certificate, view::View};

use doomstack::Top;

use std::collections::HashMap;

use talk::crypto::{
    primitives::multi::{MultiError, Signature as MultiSignature},
    Identity, KeyCard, Statement,
};

pub(crate) struct Aggregator<S: Statement> {
    view: View,
    statement: S,
    components: HashMap<Identity, MultiSignature>,
}

impl<S> Aggregator<S>
where
    S: Statement,
{
    pub fn new(view: View, statement: S) -> Self {
        Aggregator {
            view,
            statement,
            components: HashMap::new(),
        }
    }

    pub fn view(&self) -> &View {
        &self.view
    }

    pub fn statement(&self) -> &S {
        &self.statement
    }

    pub fn add(
        &mut self,
        keycard: &KeyCard,
        signature: MultiSignature,
    ) -> Result<(), Top<MultiError>> {
        #[cfg(debug_assertions)]
        {
            if !self.view.members().contains_key(&keycard.identity()) {
                panic!("Called `Aggregator::add` with foreign `KeyCard`");
            }
        }

        let identity = keycard.identity();

        signature.verify([keycard], &self.statement)?;
        self.components.insert(identity, signature);

        Ok(())
    }

    pub fn add_unchecked(
        &mut self,
        keycard: &KeyCard,
        signature: MultiSignature,
    ) -> Result<(), Top<MultiError>> {
        #[cfg(debug_assertions)]
        {
            if !self.view.members().contains_key(&keycard.identity()) {
                panic!("Called `Aggregator::add` with foreign `KeyCard`");
            }
        }

        let identity = keycard.identity();
        self.components.insert(identity, signature);

        Ok(())
    }

    pub fn check(
        &self,
        keycard: &KeyCard,
        signature: &MultiSignature,
    ) -> Result<(), Top<MultiError>> {
        #[cfg(debug_assertions)]
        {
            if !self.view.members().contains_key(&keycard.identity()) {
                panic!("Called `Aggregator::add` with foreign `KeyCard`");
            }
        }

        signature.verify([keycard], &self.statement)
    }

    pub fn multiplicity(&self) -> usize {
        self.components.len()
    }

    pub fn finalize(self) -> (S, Certificate) {
        let components = self.components.into_iter().collect::<Vec<_>>();
        let certificate = Certificate::aggregate(&self.view, components);

        (self.statement, certificate)
    }

    pub fn finalize_plurality(self) -> (S, Certificate) {
        let certificate = Certificate::aggregate_plurality(&self.view, self.components);
        (self.statement, certificate)
    }

    pub fn finalize_quorum(self) -> (S, Certificate) {
        let certificate = Certificate::aggregate_quorum(&self.view, self.components);
        (self.statement, certificate)
    }
}
