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
    pub fn genesis(genesis: &View) -> Frame {
        Frame {
            base: genesis.height(),
            highway: Vec::new(),
            metadata: Vec::new(),
            lookup: ShiftVec::new(genesis.height()),
        }
    }

    pub async fn update(&self, install: Install) -> Option<Frame> {
        let transition = install.clone().into_transition().await;

        if self.can_grow_by(&transition) || self.can_improve_by(&transition) {
            Some(self.acquire(install, transition))
        } else {
            None
        }
    }

    pub fn top(&self) -> usize {
        self.metadata
            .last()
            .map(|metadata| metadata.destination_height)
            .unwrap_or(self.base)
    }

    pub fn lookup(&self, height: usize) -> Vec<Install> {
        let height = height.clamp(self.base, self.top());

        if height < self.top() {
            self.highway[self.lookup[height]..].to_vec()
        } else {
            vec![]
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

        let mut lookup = ShiftVec::new(base);
        let mut last_tailless = 0;

        for (index, metadata) in metadata.iter().enumerate() {
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

    use crate::view::test::{generate_installs, last_installable, Client, InstallGenerator};

    async fn setup(genesis_height: usize, max_height: usize) -> (Frame, InstallGenerator) {
        let generator = InstallGenerator::new(max_height);
        let genesis = generator.view(genesis_height).await;
        let frame = Frame::genesis(&genesis);

        (frame, generator)
    }

    fn check_lookup(frame: &Frame, genesis_height: usize, expected: &[usize]) {
        for (index, expected) in expected.into_iter().enumerate() {
            assert_eq!(frame.lookup[genesis_height + index], *expected);
        }
    }

    async fn check_frame<I>(
        frame: &Frame,
        genesis_height: usize,
        tailless: I,
        generator: &InstallGenerator,
    ) where
        I: IntoIterator<Item = usize>,
    {
        for (current, last_installable) in
            last_installable(genesis_height, generator.max_height(), tailless)
                .into_iter()
                .enumerate()
                .filter(|(i, _)| *i >= genesis_height)
        {
            let mut client = Client::new(
                generator.view(last_installable).await,
                generator.view(current).await,
            );

            let installs = frame.lookup(current);

            client.update(installs).await;

            assert!(client.current().height() >= frame.top());
        }
    }

    #[tokio::test]
    async fn develop() {
        const GENESIS_HEIGHT: usize = 10;
        const MAX_HEIGHT: usize = 50;

        let (frame, generator) = setup(GENESIS_HEIGHT, MAX_HEIGHT).await;

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

        check_lookup(&f5, GENESIS_HEIGHT, expected);
        check_frame(&f5, GENESIS_HEIGHT, [25, 35, 40], &generator).await;
    }

    #[tokio::test]
    async fn all_tailless() {
        const GENESIS_HEIGHT: usize = 10;
        const MAX_HEIGHT: usize = 20;

        let (mut frame, generator) = setup(GENESIS_HEIGHT, MAX_HEIGHT).await;

        for i in 10..20 {
            let install = generator.install(i, i + 1, []).await;
            frame = frame.update(install).await.unwrap();
        }

        let expected = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

        check_lookup(&frame, GENESIS_HEIGHT, expected);
        check_frame(&frame, GENESIS_HEIGHT, 10..21, &generator).await;
    }

    #[tokio::test]
    async fn no_tailless() {
        const GENESIS_HEIGHT: usize = 10;
        const MAX_HEIGHT: usize = 21;

        let (mut frame, generator) = setup(GENESIS_HEIGHT, MAX_HEIGHT).await;

        for i in 10..20 {
            let install = generator.install(i, i + 1, [i + 2]).await;
            frame = frame.update(install).await.unwrap();
        }

        let expected = &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        check_lookup(&frame, GENESIS_HEIGHT, expected);
        check_frame(&frame, GENESIS_HEIGHT, [], &generator).await;
    }

    #[tokio::test]
    async fn new_tailless() {
        const GENESIS_HEIGHT: usize = 10;
        const MAX_HEIGHT: usize = 21;

        let (mut frame, generator) = setup(GENESIS_HEIGHT, MAX_HEIGHT).await;

        for i in 10..20 {
            let install = generator.install(i, i + 1, [i + 2]).await;
            frame = frame.update(install).await.unwrap();
        }

        let expected = &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        check_lookup(&frame, GENESIS_HEIGHT, expected);
        check_frame(&frame, GENESIS_HEIGHT, [], &generator).await;

        for i in [15, 17] {
            let install = generator.install(i - 1, i, []).await;
            frame = frame.update(install).await.unwrap();
        }

        let expected = &[0, 0, 0, 0, 0, 5, 5, 7, 7, 7];

        check_lookup(&frame, GENESIS_HEIGHT, expected);
        check_frame(&frame, GENESIS_HEIGHT, [15, 17], &generator).await;
    }

    #[tokio::test]
    async fn shortcut_tailless() {
        const GENESIS_HEIGHT: usize = 10;
        const MAX_HEIGHT: usize = 21;

        let (mut frame, generator) = setup(GENESIS_HEIGHT, MAX_HEIGHT).await;

        let i0 = generator.install(10, 11, [12, 13]).await;
        frame = frame.update(i0).await.unwrap();

        let i1 = generator.install(11, 12, [13]).await;
        frame = frame.update(i1).await.unwrap();

        let i2 = generator.install(12, 13, []).await;
        frame = frame.update(i2).await.unwrap();

        let i3 = generator.install(13, 14, [15]).await;
        frame = frame.update(i3).await.unwrap();

        let expected = &[0, 0, 0, 3];

        check_lookup(&frame, GENESIS_HEIGHT, expected);
        check_frame(&frame, GENESIS_HEIGHT, [13], &generator).await;

        let i4 = generator.install(10, 12, []).await;
        frame = frame.update(i4).await.unwrap();

        let expected = &[0, 0, 1, 2];

        check_lookup(&frame, GENESIS_HEIGHT, expected);
        check_frame(&frame, GENESIS_HEIGHT, [12, 13], &generator).await;
    }

    #[tokio::test]
    async fn shortcut_tails() {
        const GENESIS_HEIGHT: usize = 10;
        const MAX_HEIGHT: usize = 21;

        let (mut frame, generator) = setup(GENESIS_HEIGHT, MAX_HEIGHT).await;

        let i0 = generator.install(10, 11, [12, 13]).await;
        frame = frame.update(i0).await.unwrap();

        let i1 = generator.install(11, 12, [13]).await;
        frame = frame.update(i1).await.unwrap();

        let i2 = generator.install(12, 13, []).await;
        frame = frame.update(i2).await.unwrap();

        let i3 = generator.install(13, 14, [15]).await;
        frame = frame.update(i3).await.unwrap();

        let expected = &[0, 0, 0, 3];

        check_lookup(&frame, GENESIS_HEIGHT, expected);
        check_frame(&frame, GENESIS_HEIGHT, [13], &generator).await;

        let i4 = generator.install(10, 12, [13]).await;
        frame = frame.update(i4).await.unwrap();

        let expected = &[0, 0, 0, 2];

        check_lookup(&frame, GENESIS_HEIGHT, expected);
        check_frame(&frame, GENESIS_HEIGHT, [13], &generator).await;
    }

    #[tokio::test]
    #[ignore]
    async fn stress_light_checks() {
        const GENESIS_HEIGHT: usize = 10;
        const MAX_HEIGHT: usize = 100; // 100 ~= 2 seconds, 500 ~= 65 seconds

        let (mut frame, generator) = setup(GENESIS_HEIGHT, MAX_HEIGHT).await;
        let installs =
            generate_installs(GENESIS_HEIGHT, MAX_HEIGHT, MAX_HEIGHT / 5, MAX_HEIGHT / 15).await;

        let mut tailless = Vec::new();
        for (source, destination, tail) in installs.into_iter() {
            if tail.len() == 0 {
                tailless.push(destination);
            }
            let install = generator
                .install_dummy_certificate(source, destination, tail)
                .await;

            if let Some(new) = frame.update(install).await {
                frame = new;
            }
        }

        assert_eq!(frame.top(), MAX_HEIGHT - 1);
        check_frame(&frame, GENESIS_HEIGHT, tailless, &generator).await;
    }

    #[tokio::test]
    #[ignore]
    async fn stress_heavy_checks() {
        const GENESIS_HEIGHT: usize = 10;
        const MAX_HEIGHT: usize = 100; // 100 ~= 14 seconds

        let (mut frame, generator) = setup(GENESIS_HEIGHT, MAX_HEIGHT).await;
        let installs =
            generate_installs(GENESIS_HEIGHT, MAX_HEIGHT, MAX_HEIGHT / 5, MAX_HEIGHT / 15).await;

        let mut tailless = Vec::new();
        for (source, destination, tail) in installs.into_iter() {
            if tail.len() == 0 {
                tailless.push(destination);
            }
            let install = generator
                .install_dummy_certificate(source, destination, tail)
                .await;

            if let Some(new) = frame.update(install).await {
                frame = new;
                check_frame(&frame, GENESIS_HEIGHT, tailless.iter().cloned(), &generator).await;
            }
        }

        assert_eq!(frame.top(), MAX_HEIGHT - 1);
    }
}
