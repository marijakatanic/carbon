use std::collections::BTreeSet;

use talk::crypto::primitives::hash::{Hash, Hasher};

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
