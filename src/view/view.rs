use crate::{
    crypto::Identify,
    view::{Change, Increment, FAMILY, VIEWS},
};

use doomstack::{here, Doom, ResultExt, Top};

use std::collections::hash_map::Entry;
use std::collections::BTreeMap;
use std::sync::Arc;

use talk::crypto::primitives::hash::Hash;
use talk::crypto::Identity;
use talk::crypto::KeyCard;

use zebra::database::{Collection, CollectionTransaction};

#[derive(Clone)]
pub(crate) struct View {
    data: Arc<Data>,
}

struct Data {
    height: usize,
    changes: Collection<Change>,
    members: BTreeMap<Identity, KeyCard>,
}

#[derive(Doom)]
pub(crate) enum ViewError {
    #[doom(description("Extension results in a member joining more than once"))]
    DoubleJoin,
    #[doom(description("Extension results in a member leaving before joining"))]
    UnmatchedLeave,
    #[doom(description("Extension results in a member leaving more than once"))]
    DoubleLeave,
}

impl View {
    pub fn genesis<M>(members: M) -> Self
    where
        M: IntoIterator<Item = KeyCard>,
    {
        let members = members
            .into_iter()
            .map(|keycard| (keycard.identity(), keycard))
            .collect::<BTreeMap<_, _>>();

        #[cfg(debug_assertions)]
        {
            if members.len() < 4 {
                panic!("called `View::genesis` with insufficient `members` for Byzantine resilience (i.e., 4)");
            }
        }

        let height = members.len();

        let increment = members
            .values()
            .cloned()
            .map(|replica| Change::Join(replica));

        let mut changes = FAMILY.empty_collection();
        let mut transaction = CollectionTransaction::new();

        for change in increment {
            transaction.insert(change).unwrap();
        }

        changes.execute(transaction);

        let identifier = changes.commit();

        let data = Arc::new(Data {
            height,
            changes,
            members,
        });

        let view = View { data };

        match VIEWS.lock().unwrap().entry(identifier) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => entry.insert(view).clone(),
        }
    }

    pub fn extend(&self, increment: Increment) -> Self {
        #[cfg(debug_assertions)]
        {
            for change in increment.iter() {
                self.validate_extension(change)
                    .expect("called `extend` with an invalid extension");
            }
        }

        let height = self.data.height + increment.len();

        let mut changes = self.data.changes.clone();
        let mut transaction = CollectionTransaction::new();

        for change in increment.iter().cloned() {
            transaction.insert(change).unwrap();
        }

        changes.execute(transaction);

        let identifier = changes.commit();

        let mut members = self.data.members.clone();

        for change in increment {
            match change {
                Change::Join(replica) => {
                    members.insert(replica.identity(), replica);
                }
                Change::Leave(replica) => {
                    members.remove(&replica.identity());
                }
            }
        }

        let data = Arc::new(Data {
            height,
            changes,
            members,
        });

        let view = View { data };

        match VIEWS.lock().unwrap().entry(identifier) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => entry.insert(view).clone(),
        }
    }

    pub fn get(identifier: Hash) -> Option<Self> {
        VIEWS.lock().unwrap().get(&identifier).cloned()
    }

    pub fn height(&self) -> usize {
        self.data.height
    }

    pub fn plurality(&self) -> usize {
        (self.data.members.len() - 1) / 3 + 1
    }

    pub fn quorum(&self) -> usize {
        self.data.members.len() - (self.data.members.len() - 1) / 3
    }

    pub fn members(&self) -> &BTreeMap<Identity, KeyCard> {
        &self.data.members
    }

    pub fn validate_extension(&self, change: &Change) -> Result<(), Top<ViewError>> {
        let join = Change::Join(change.keycard());
        let leave = Change::Leave(change.keycard());

        let mut transaction = CollectionTransaction::new();

        let join_query = transaction.contains(&join).unwrap();
        let leave_query = transaction.contains(&leave).unwrap();

        let response = self.data.changes.clone().execute(transaction);

        match change {
            Change::Join(_) => {
                if response.contains(&join_query) {
                    ViewError::DoubleJoin.fail().spot(here!())
                } else {
                    Ok(())
                }
            }
            Change::Leave(_) => {
                if !response.contains(&join_query) {
                    ViewError::UnmatchedLeave.fail().spot(here!())
                } else if response.contains(&leave_query) {
                    ViewError::DoubleLeave.fail().spot(here!())
                } else {
                    Ok(())
                }
            }
        }
    }
}

impl Identify for View {
    fn identifier(&self) -> Hash {
        self.data.changes.identifier()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::iter;

    use talk::crypto::{KeyCard, KeyChain};

    fn random_keycards(count: usize) -> Vec<KeyCard> {
        iter::repeat_with(|| KeyChain::random().keycard())
            .take(count)
            .collect()
    }

    #[test]
    #[should_panic]
    fn empty() {
        let _ = View::genesis([]);
    }

    #[test]
    #[should_panic]
    fn too_few() {
        let alice = KeyChain::random().keycard();
        let bob = KeyChain::random().keycard();
        let carl = KeyChain::random().keycard();

        let _ = View::genesis([alice, bob, carl]);
    }

    #[test]
    #[should_panic]
    fn repeating() {
        let alice = KeyChain::random().keycard();
        let bob = KeyChain::random().keycard();
        let carl = KeyChain::random().keycard();

        let _ = View::genesis([alice.clone(), bob, carl, alice]);
    }

    #[test]
    #[should_panic]
    fn unmatched_leave() {
        let view = View::genesis(random_keycards(16));

        let increment =
            std::collections::BTreeSet::from([Change::Leave(KeyChain::random().keycard())]);

        let _ = view.extend(increment);
    }

    #[test]
    fn genesis_height() {
        for height in 4..32 {
            let view = View::genesis(random_keycards(height));
            assert_eq!(view.height(), height);
        }
    }

    #[test]
    fn extended_join_counters() {
        let mut view = View::genesis(random_keycards(4));

        for step in 0..16 {
            let increment = random_keycards(4)
                .into_iter()
                .map(|keycard| Change::Join(keycard))
                .collect();

            view = view.extend(increment);

            assert_eq!(view.height(), 4 * (step + 2));
            assert_eq!(view.members().len(), 4 * (step + 2));
        }
    }

    #[test]
    fn extended_join_leave_counters() {
        let mut view = View::genesis(random_keycards(4));

        let mut steps = Vec::new();

        for _ in 0..16 {
            let keycards = random_keycards(4);

            let increment = keycards
                .iter()
                .cloned()
                .map(|keycard| Change::Join(keycard))
                .collect();

            view = view.extend(increment);
            steps.push(keycards);
        }

        for (index, step) in steps.into_iter().enumerate() {
            let increment = step
                .into_iter()
                .map(|keycard| Change::Leave(keycard))
                .collect();

            view = view.extend(increment);

            assert_eq!(view.height(), 4 * (18 + index));
            assert_eq!(view.members().len(), 4 * (16 - index));
        }
    }

    #[test]
    fn identifier_associativity() {
        let keycards = random_keycards(32);

        let joins = keycards
            .iter()
            .cloned()
            .map(|keycard| Change::Join(keycard))
            .collect::<Vec<_>>();

        let direct = View::genesis(keycards[0..16].to_vec());

        let two_steps = View::genesis(keycards[0..8].to_vec())
            .extend(joins[8..16].into_iter().cloned().collect());

        let four_steps = View::genesis(keycards[0..4].to_vec())
            .extend(joins[4..8].into_iter().cloned().collect())
            .extend(joins[8..12].into_iter().cloned().collect())
            .extend(joins[12..16].into_iter().cloned().collect());

        assert_eq!(two_steps.identifier(), direct.identifier());
        assert_eq!(four_steps.identifier(), direct.identifier());
    }
}
