use crate::view::{Change, Increment, Install, InstallAggregator, View};

use talk::crypto::{KeyCard, KeyChain};

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

    pub fn max_height(&self) -> usize {
        self.keychains.len()
    }

    pub async fn view(&self, height: usize) -> View {
        let members = self.keycards[0..height].iter().cloned().collect::<Vec<_>>();
        View::genesis(members).await
    }

    pub async fn install<T>(&self, source: usize, destination: usize, tail: T) -> Install
    where
        T: IntoIterator<Item = usize>,
    {
        let increments = self.increments(source, destination, tail);
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

    /// This creates an install message with an invalid certificate
    /// in O(1) time instead of O(N), where N is the number of view members.
    /// 
    /// `InstallGenerator::install` should be preferred for small N or small
    /// number of calls to the method (small total complexity).
    /// 
    /// This method is ONLY supposed to be used for testing functionality 
    /// that assumes that install messages were correctly produced.
    /// Otherwise, it will likely result in a panic. See `Install::dummy` for
    /// more information.
    pub async fn install_dummy<T>(&self, source: usize, destination: usize, tail: T) -> Install
    where
        T: IntoIterator<Item = usize>,
    {
        let increments = self.increments(source, destination, tail);
        let source = self.view(source).await;

        Install::dummy(&source, increments)
    }

    fn increments<T>(&self, source: usize, destination: usize, tail: T) -> Vec<Increment>
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

        increments
    }
}
