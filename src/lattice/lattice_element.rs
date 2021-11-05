use crate::{discovery::Client, view::View};

use doomstack::{Doom, Top};

use talk::unicast::Message;

pub(crate) trait LatticeElement: Message + Clone {
    fn validate(&self, client: &Client, view: &View) -> Result<(), Top<LatticeElementError>>;
}

#[derive(Doom)]
pub(crate) enum LatticeElementError {
    #[doom(description("Lattice element invalid"))]
    ElementInvalid,
}
