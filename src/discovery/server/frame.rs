use crate::{
    data::ShiftVec,
    view::{Install, Transition, View},
};

pub(in crate::discovery::server) struct Frame {
    base: usize,
    highway: Vec<Install>,
    metadata: Vec<Metadata>,
    lookup: ShiftVec<usize>,
}

#[derive(Clone)]
struct Metadata {
    source_height: usize,
    destination_height: usize,
    tailless: bool,
}

impl Frame {
    fn genesis(genesis: View) -> Frame {
        Frame {
            base: genesis.height(),
            highway: Vec::new(),
            metadata: Vec::new(),
            lookup: ShiftVec::new(genesis.height()),
        }
    }

    async fn update(&self, install: Install) -> Option<Frame> {
        let transition = install.clone().into_transition().await;

        if self.can_grow_by(&transition) || self.can_improve_by(&transition) {
            Some(self.acquire(install, transition))
        } else {
            None
        }
    }

    fn acquire(&self, install: Install, transition: Transition) -> Frame {
        let base = self.base;

        let mut highway = Vec::new();
        let mut metadata = Vec::new();

        if let Some(to) = self.locate_by_destination(transition.source().height()) {
            highway.extend_from_slice(&self.highway[..=to]);
            metadata.extend_from_slice(&self.metadata[..=to]);
        }

        highway.push(install);

        metadata.push(Metadata {
            source_height: transition.source().height(),
            destination_height: transition.destination().height(),
            tailless: transition.tailless(),
        });

        if let Some(from) = self.locate_by_source(transition.destination().height()) {
            highway.extend_from_slice(&self.highway[from..]);
            metadata.extend_from_slice(&self.metadata[from..]);
        }

        let mut lookup = ShiftVec::new(self.base);
        let mut last_tailless = 0;

        for (index, metadata) in self.metadata.iter().enumerate() {
            if metadata.tailless {
                while lookup.len() < metadata.destination_height {
                    lookup.push(last_tailless)
                }

                last_tailless = index + 1;
            }
        }

        let top = metadata.last().unwrap().destination_height;

        while lookup.len() < top {
            lookup.push(last_tailless);
        }

        Self {
            base,
            highway,
            metadata,
            lookup,
        }
    }

    fn can_grow_by(&self, transition: &Transition) -> bool {
        transition.destination().height() > self.top()
    }

    fn can_improve_by(&self, transition: &Transition) -> bool {
        if let (Some(source), Some(destination)) = (
            self.locate_by_source(transition.source().height()),
            self.locate_by_destination(transition.destination().height()),
        ) {
            (source < destination)
                || (transition.tailless() && !self.metadata[destination].tailless)
        } else {
            false
        }
    }

    fn top(&self) -> usize {
        self.metadata
            .last()
            .map(|metadata| metadata.destination_height)
            .unwrap_or(self.base)
    }

    fn locate_by_source(&self, height: usize) -> Option<usize> {
        self.metadata
            .binary_search_by_key(&height, |metadata| metadata.source_height)
            .ok()
    }

    fn locate_by_destination(&self, height: usize) -> Option<usize> {
        self.metadata
            .binary_search_by_key(&height, |metadata| metadata.destination_height)
            .ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::view::test::InstallGenerator;

    #[tokio::test]
    async fn develop() {
        let generator = InstallGenerator::new(50);
        let genesis = generator.view(10).await;

        let frame = Frame::genesis(genesis);

        let i0 = generator.install(10, 15, [16]).await;
        let f0 = frame.update(i0).await.unwrap();

        let i1 = generator.install(15, 20, [21]).await;
        let f1 = f0.update(i1).await.unwrap();

        let i2 = generator.install(20, 25, []).await;
        let f2 = f1.update(i2).await.unwrap();

        let i3 = generator.install(25, 30, [31]).await;
        let f3 = f2.update(i3).await.unwrap();

        let i4 = generator.install(30, 35, []).await;
        let f4 = f3.update(i4).await.unwrap();

        let i5 = generator.install(35, 40, []).await;
        let f5 = f4.update(i5).await.unwrap();

        let expected = &[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 5, 5, 5, 5,
            5,
        ];

        for (index, expected) in expected.into_iter().enumerate() {
            assert_eq!(f5.lookup[10 + index], *expected);
        }
    }
}
