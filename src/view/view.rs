use crate::{
    crypto::Identify,
    view::{Change, Increment, FAMILY, VIEWS},
};

use std::collections::hash_map::Entry;
use std::collections::HashSet;
use std::sync::Arc;

use talk::crypto::primitives::hash::Hash;
use talk::crypto::KeyCard;

use zebra::database::{Collection, CollectionTransaction};

#[derive(Clone)]
pub(crate) struct View {
    data: Arc<Data>,
}

struct Data {
    height: usize,
    changes: Collection<Change>,
    members: Vec<KeyCard>,
}

impl View {
    pub async fn genesis<M>(members: M) -> Self
    where
        M: IntoIterator<Item = KeyCard>,
    {
        let mut members = members.into_iter().collect::<Vec<_>>();
        members.sort_by_key(KeyCard::identity);

        #[cfg(debug_assertions)]
        {
            // Verify that all `members` are distinct, and sufficiently many for Byzantine resilience.

            let members_set = members.clone().into_iter().collect::<HashSet<_>>();

            if members_set.len() > members.len() {
                panic!("called `View::genesis` with non-distinct `members`");
            }

            if members.len() < 4 {
                panic!("called `View::genesis` with insufficient `members` for Byzantine resilience (i.e., 4)");
            }
        }

        let height = members.len();

        let updates = members
            .clone()
            .into_iter()
            .map(|replica| Change::Join(replica));

        let mut changes = FAMILY.empty_collection();
        let mut transaction = CollectionTransaction::new();

        for update in updates {
            transaction.insert(update).unwrap();
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

    pub async fn extend(&self, increment: Increment) -> Self {
        let updates = increment.into_vec();

        #[cfg(debug_assertions)]
        {
            // Verify that no element of `updates` is already in `self.data.changes`,
            // and that all negative changes of `updates` are matched by a corresponding
            // positive change in `self.data.changes`.
            //
            // (Note that all identities in `updates` are already guaranteed to be distinct)

            use std::collections::HashMap;

            let requirements = updates
                .iter()
                .filter_map(Change::requirement)
                .collect::<Vec<_>>();

            // These are all guaranteed to be distinct: indeed, all identities in `updates`
            // are distinct, and `requirements` is a mirror of a subsequence in `updates`
            // (`Change::Leave`s are mapped onto corresponding `Change::Join`s).
            let queries = updates.clone().into_iter().chain(requirements.into_iter());

            let mut transaction = CollectionTransaction::new();

            let queries = queries
                .map(|change| {
                    let query = transaction.contains(&change).unwrap();
                    (change, query)
                })
                .collect::<Vec<_>>();

            let response = self.data.changes.clone().execute(transaction);

            let response = queries
                .into_iter()
                .map(|(change, query)| (change, response.contains(&query)))
                .collect::<HashMap<_, _>>();

            for update in updates.iter() {
                // Verify that `update` is not already in `self.data.changes`
                if response[update] {
                    panic!("called `View::extend` with a pre-existing `Change`");
                }

                // Verify that, if `update` is negative, its positive mirror is in `self.data.changes`
                if let Some(requirement) = update.requirement() {
                    if !response[&requirement] {
                        panic!("called `View::extend` with an unsatisfied requirement (unmatched `Change::Leave`)");
                    }
                }
            }
        }

        let height = self.data.height + updates.len();

        let mut changes = self.data.changes.clone();
        let mut transaction = CollectionTransaction::new();

        for update in updates.clone() {
            transaction.insert(update).unwrap();
        }

        changes.execute(transaction);

        let identifier = changes.commit();

        let mut members = self
            .data
            .members
            .clone()
            .into_iter()
            .collect::<HashSet<_>>();

        for update in updates {
            match update {
                Change::Join(replica) => {
                    members.insert(replica);
                }
                Change::Leave(replica) => {
                    members.remove(&replica);
                }
            }
        }

        let mut members = members.into_iter().collect::<Vec<_>>();
        members.sort_by_key(KeyCard::identity);

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

    pub fn members(&self) -> &[KeyCard] {
        self.data.members.as_slice()
    }
}

impl Identify for View {
    fn identifier(&self) -> Hash {
        self.data.changes.commit()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::iter;

    use talk::crypto::{KeyCard, KeyChain};

    fn random_keycards(count: usize) -> Vec<KeyCard> {
        iter::repeat_with(|| KeyChain::random().keycard())
            .take(count)
            .collect()
    }

    #[tokio::test]
    #[should_panic]
    async fn empty() {
        let _ = View::genesis([]).await;
    }

    #[tokio::test]
    #[should_panic]
    async fn too_few() {
        let alice = KeyChain::random().keycard();
        let bob = KeyChain::random().keycard();
        let carl = KeyChain::random().keycard();

        let _ = View::genesis([alice, bob, carl]).await;
    }

    #[tokio::test]
    #[should_panic]
    async fn repeating() {
        let alice = KeyChain::random().keycard();
        let bob = KeyChain::random().keycard();
        let carl = KeyChain::random().keycard();

        let _ = View::genesis([alice.clone(), bob, carl, alice]).await;
    }

    #[tokio::test]
    #[should_panic]
    async fn unmatched_leave() {
        let view = View::genesis(random_keycards(16)).await;
        let increment = Increment::new([Change::Leave(KeyChain::random().keycard())]);
        let _ = view.extend(increment).await;
    }

    #[tokio::test]
    async fn genesis_height() {
        for height in 4..32 {
            let view = View::genesis(random_keycards(height)).await;
            assert_eq!(view.height(), height);
        }
    }

    #[tokio::test]
    async fn extended_join_counters() {
        let mut view = View::genesis(random_keycards(4)).await;

        for step in 0..16 {
            let increment = Increment::new(
                random_keycards(4)
                    .into_iter()
                    .map(|keycard| Change::Join(keycard)),
            );

            view = view.extend(increment).await;

            assert_eq!(view.height(), 4 * (step + 2));
            assert_eq!(view.members().len(), 4 * (step + 2));
        }
    }

    #[tokio::test]
    async fn extended_join_leave_counters() {
        let mut view = View::genesis(random_keycards(4)).await;

        let mut steps = Vec::new();

        for _ in 0..16 {
            let keycards = random_keycards(4);

            let increment = Increment::new(
                keycards
                    .iter()
                    .cloned()
                    .map(|keycard| Change::Join(keycard)),
            );

            view = view.extend(increment).await;
            steps.push(keycards);
        }

        for (index, step) in steps.into_iter().enumerate() {
            let increment = Increment::new(step.into_iter().map(|keycard| Change::Leave(keycard)));
            view = view.extend(increment).await;

            assert_eq!(view.height(), 4 * (18 + index));
            assert_eq!(view.members().len(), 4 * (16 - index));
        }
    }

    #[tokio::test]
    async fn identifier_associativity() {
        let keycards = random_keycards(32);

        let joins = keycards
            .iter()
            .cloned()
            .map(|keycard| Change::Join(keycard))
            .collect::<Vec<_>>();

        let direct = View::genesis(keycards[0..16].to_vec()).await;

        let two_steps = View::genesis(keycards[0..8].to_vec())
            .await
            .extend(Increment::new(joins[8..16].to_vec()))
            .await;

        let four_steps = View::genesis(keycards[0..4].to_vec())
            .await
            .extend(Increment::new(joins[4..8].to_vec()))
            .await
            .extend(Increment::new(joins[8..12].to_vec()))
            .await
            .extend(Increment::new(joins[12..16].to_vec()))
            .await;

        assert_eq!(two_steps.identifier(), direct.identifier());
        assert_eq!(four_steps.identifier(), direct.identifier());
    }
}
