use crate::{account::Id, view::View};

use std::ops::Range;

use talk::crypto::Identity;

impl View {
    pub fn allocation_range(&self, allocator: Identity) -> Range<Id> {
        let index =
            self.members()
                .keys()
                .enumerate()
                .find_map(|(index, identity)| {
                    if *identity == allocator {
                        Some(index)
                    } else {
                        None
                    }
                })
                .expect("this `View` does not contain the provided `allocator`") as u64;

        let width = u64::MAX / self.members().len() as u64;
        let start = index * width;
        let end = (index + 1) * width;

        start..end
    }
}
