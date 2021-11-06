use crate::{discovery::Client, view::View};

use doomstack::{Doom, Top};

use talk::crypto::primitives::hash;
use talk::crypto::primitives::hash::Hash;
use talk::unicast::Message;

pub(crate) trait Element: Message + Clone {
    fn validate(&self, client: &Client, view: &View) -> Result<(), Top<ElementError>>;

    fn identifier(&self) -> Hash {
        hash::hash(self).unwrap()
    }
}

#[derive(Doom)]
pub(crate) enum ElementError {
    #[doom(description("Lattice element invalid"))]
    ElementInvalid,
}
