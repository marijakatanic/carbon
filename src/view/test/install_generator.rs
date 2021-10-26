use crate::view::{Change, Increment, Install, InstallAggregator, View};

use talk::crypto::{KeyCard, KeyChain};

use zebra::Commitment;

pub(crate) struct InstallGenerator {
    keychains: Vec<KeyChain>,
    keycards: Vec<KeyCard>,
}

impl InstallGenerator {
    pub fn new(views: usize) -> InstallGenerator {
        let keychains = (0..views).map(|_| KeyChain::random()).collect::<Vec<_>>();
        let keycards = keychains.iter().map(KeyChain::keycard).collect::<Vec<_>>();

        InstallGenerator {
            keychains,
            keycards,
        }
    }

    pub async fn view(&self, height: usize) -> View {
        let members = self.keycards[0..height].iter().cloned().collect::<Vec<_>>();
        View::genesis(members).await
    }

    pub async fn install<T>(&self, source: usize, destination: usize, tail: T) -> Install
    where
        T: IntoIterator<Item = usize>,
    {
        let mut heights = vec![source, destination];
        heights.extend(tail);

        let increments = heights
            .windows(2)
            .map(|window| {
                Increment::new(
                    self.keycards[window[0]..window[1]]
                        .iter()
                        .cloned()
                        .map(|replica| Change::Join(replica))
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();

        let source = self.view(source).await;
        let mut aggregator = InstallAggregator::new(source.clone(), increments.clone());

        for (keychain, keycard) in self
            .keychains
            .iter()
            .zip(self.keycards.iter())
            .take(source.plurality())
        {
            let signature = Install::certify(keychain, &source, increments.clone());
            aggregator.add(keycard, signature).unwrap();
        }

        aggregator.finalize()
    }
}
