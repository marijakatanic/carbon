use serde::Serialize;

use std::collections::BTreeSet;

use talk::crypto::primitives::hash::{Hash, Hasher};

use zebra::database::Collection;

/// ## `Identify` and `Eq`
///
/// When implementing both `Identify` and [`Eq`], it is important that the following
/// property holds:
///
/// ```text
/// e1 == e2 <-> e1.identifier() == e2.identifier()
/// ```
///
/// In other words, two elements are equal if and only if their identifiers are equal.
pub trait Identify {
    fn identifier(&self) -> Hash;
}

impl Identify for Hash {
    fn identifier(&self) -> Hash {
        *self
    }
}

impl<T> Identify for &T
where
    T: Identify,
{
    fn identifier(&self) -> Hash {
        (*self).identifier()
    }
}

impl<A, B> Identify for (A, B)
where
    A: Identify,
    B: Identify,
{
    fn identifier(&self) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(&self.0.identifier()).unwrap();
        hasher.update(&self.1.identifier()).unwrap();
        hasher.finalize()
    }
}

impl<A, B, C> Identify for (A, B, C)
where
    A: Identify,
    B: Identify,
    C: Identify,
{
    fn identifier(&self) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(&self.0.identifier()).unwrap();
        hasher.update(&self.1.identifier()).unwrap();
        hasher.update(&self.2.identifier()).unwrap();
        hasher.finalize()
    }
}

impl<A, B, C, D> Identify for (A, B, C, D)
where
    A: Identify,
    B: Identify,
    C: Identify,
    D: Identify,
{
    fn identifier(&self) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(&self.0.identifier()).unwrap();
        hasher.update(&self.1.identifier()).unwrap();
        hasher.update(&self.2.identifier()).unwrap();
        hasher.update(&self.3.identifier()).unwrap();
        hasher.finalize()
    }
}

impl<T> Identify for Collection<T>
where
    T: 'static + Serialize + Send + Sync,
{
    fn identifier(&self) -> Hash {
        self.commit()
    }
}

impl<T> Identify for Vec<T>
where
    T: Identify,
{
    fn identifier(&self) -> Hash {
        let mut hasher = Hasher::new();

        for element in self.iter() {
            hasher.update(&element.identifier()).unwrap();
        }

        hasher.finalize()
    }
}

impl<T> Identify for BTreeSet<T>
where
    T: Identify,
{
    fn identifier(&self) -> Hash {
        let mut hasher = Hasher::new();

        for element in self.iter() {
            hasher.update(&element.identifier()).unwrap();
        }

        hasher.finalize()
    }
}
