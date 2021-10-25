use crate::view::{Increment, View};

use zebra::Commitment;

pub(crate) struct Transition {
    source: View,
    destination: View,
    tail: Vec<View>,
}

impl Transition {
    pub(in crate::view) async fn new(source: Commitment, increments: Vec<Increment>) -> Self {
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
}
