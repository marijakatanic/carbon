use crate::{crypto::Identify, discovery::Client, view::View};

use doomstack::{Doom, Top};

use talk::unicast::Message;

pub(crate) trait Element: Message + Identify + Clone {
    fn validate(&self, client: &Client, view: &View) -> Result<(), Top<ElementError>>;
}

#[derive(Doom)]
pub(crate) enum ElementError {
    #[doom(description("Lattice element invalid"))]
    ElementInvalid,
}
