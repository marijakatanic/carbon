use crate::{
    crypto::{Aggregator, Certificate, Header},
    view::{Increment, Transition, View},
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::de;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use talk::crypto::primitives::multi::{MultiError, Signature as MultiSignature};
use talk::crypto::{KeyCard, KeyChain, Statement as CryptoStatement};

use zebra::Commitment;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub(crate) struct Install {
    statement: Statement,
    certificate: Certificate,
}

pub(crate) struct InstallAggregator(Aggregator<Statement>);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Statement {
    source: Commitment,
    increments: Vec<Increment>,
}

#[derive(Doom)]
pub(crate) enum InstallError {
    #[doom(description("Source view unknown"))]
    SourceUnknown,
    #[doom(description("Certificate invalid"))]
    CertificateInvalid,
}

impl Install {
    pub fn certify<I>(keychain: &KeyChain, source: &View, increments: I) -> MultiSignature
    where
        I: IntoIterator<Item = Increment>,
    {
        let increments = increments.into_iter().collect::<Vec<_>>();

        let statement = Statement {
            source: source.identifier(),
            increments,
        };

        keychain
            .multisign(&statement)
            .expect("Panic at `Install::certify`: unexpected error from `keychain.multisign`")
    }

    pub async fn into_transition(self) -> Transition {
        Transition::new(self.statement.source, self.statement.increments).await
    }

    fn check(&self) -> Result<(), Top<InstallError>> {
        let source = View::get(self.statement.source)
            .ok_or(InstallError::SourceUnknown.into_top())
            .spot(here!())?;

        self.certificate
            .verify_plurality(&source, &self.statement)
            .pot(InstallError::CertificateInvalid, here!())?;

        #[cfg(debug_assertions)]
        {
            if self.statement.increments.len() == 0 {
                panic!("An `Install` message was generated with no increments");
            }
        }

        Ok(())
    }
}

impl InstallAggregator {
    pub fn new<I>(source: View, increments: I) -> Self
    where
        I: IntoIterator<Item = Increment>,
    {
        let statement = Statement {
            source: source.identifier(),
            increments: increments.into_iter().collect::<Vec<_>>(),
        };

        InstallAggregator(Aggregator::new(source, statement))
    }

    pub fn add(
        &mut self,
        keycard: &KeyCard,
        signature: MultiSignature,
    ) -> Result<(), Top<MultiError>> {
        self.0.add(keycard, signature)
    }

    pub fn finalize(self) -> Install {
        let (statement, certificate) = self.0.finalize_plurality();

        Install {
            statement,
            certificate,
        }
    }
}

impl Serialize for Install {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Install::serialize(&self, serializer)
    }
}

impl<'de> Deserialize<'de> for Install {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let install = Install::deserialize(deserializer)?;
        install.check().map_err(|err| de::Error::custom(err))?;
        Ok(install)
    }
}

impl CryptoStatement for Statement {
    type Header = Header;
    const HEADER: Header = Header::Install;
}
