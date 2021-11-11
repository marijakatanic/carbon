use talk::crypto::primitives::hash::Hash;

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
