use crate::{discovery::Client, view::View};

use doomstack::{Doom, Top};

use talk::unicast::Message;

pub(crate) trait Element: Message + Clone {
    fn validate(&self, client: &Client, view: &View) -> Result<(), Top<ElementError>>;
}

#[derive(Doom)]
pub(crate) enum ElementError {
    #[doom(description("Lattice element invalid"))]
    ElementInvalid,
}
