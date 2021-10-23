use crate::{
    crypto::Header,
    view::{Certificate, Increment, View},
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::de;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use talk::crypto::Statement;

use zebra::Commitment;

#[derive(Serialize, Deserialize)]
#[serde(remote = "Self")]
pub(crate) struct Install {
    payload: Payload,
    certificate: Certificate,
}

#[derive(Serialize, Deserialize)]
struct Payload {
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
    fn check(&self) -> Result<(), Top<InstallError>> {
        let source = View::get(self.payload.source)
            .ok_or(InstallError::SourceUnknown.into_top())
            .spot(here!())?;

        self.certificate
            .verify_plurality(&source, &self.payload)
            .pot(InstallError::CertificateInvalid, here!())
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

impl Statement for Payload {
    type Header = Header;
    const HEADER: Header = Header::Install;
}
