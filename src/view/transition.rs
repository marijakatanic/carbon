use crate::view::{Increment, View};

use talk::crypto::primitives::hash::Hash;

#[derive(Clone)]
pub(crate) struct Transition {
    source: View,
    destination: View,
    tail: Vec<View>,
}

impl Transition {
    pub(in crate::view) async fn new(source: Hash, increments: Vec<Increment>) -> Self {
        let source =
            View::get(source).expect("An `Install` message was accepted with unknown `source`");

        let mut increments = increments.into_iter();

        let destination = source
            .extend(
                increments
                    .next()
                    .expect("An `Install` message was accepted with no increments"),
            )
            .await;

        let mut tail = Vec::new();
        let mut head = destination.clone();

        for increment in increments {
            head = head.extend(increment).await;
            tail.push(head.clone());
        }

        Transition {
            source,
            destination,
            tail,
        }
    }

    pub fn source(&self) -> &View {
        &self.source
    }

    pub fn destination(&self) -> &View {
        &self.destination
    }

    pub fn tail(&self) -> &[View] {
        self.tail.as_slice()
    }

    pub fn tailless(&self) -> bool {
        self.tail.len() == 0
    }
}
