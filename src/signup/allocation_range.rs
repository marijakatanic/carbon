use crate::{account::Id, view::View};

use std::ops::Range;

use talk::crypto::Identity;

impl View {
    pub fn allocation_range(&self, assigner: Identity) -> Range<Id> {
        let index =
            self.members()
                .keys()
                .enumerate()
                .find_map(|(index, identity)| {
                    if *identity == assigner {
                        Some(index)
                    } else {
                        None
                    }
                })
                .expect("this `View` does not contain the provided `assigner`") as u64;

        let width = u64::MAX / self.members().len() as u64;
        let start = index * width;
        let end = (index + 1) * width;

        start..end
    }
}
