use doomstack::{Doom, Top};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{BufWriter, Write as _},
};

#[derive(Doom)]
pub enum ParametersError {
    #[doom(description("Fail"))]
    Fail,
}

pub trait Export: Serialize + DeserializeOwned {
    fn read(path: &str) -> Result<Self, Top<ParametersError>> {
        let reader = || -> Result<Self, std::io::Error> {
            let data = fs::read(path)?;
            Ok(serde_json::from_slice(data.as_slice())?)
        };
        reader().map_err(|_| ParametersError::Fail.into_top())
    }

    fn write(&self, path: &str) -> Result<(), Top<ParametersError>> {
        let writer = || -> Result<(), std::io::Error> {
            let file = OpenOptions::new().create(true).write(true).open(path)?;
            let mut writer = BufWriter::new(file);
            let data = serde_json::to_string_pretty(self).unwrap();
            writer.write_all(data.as_ref())?;
            writer.write_all(b"\n")?;
            Ok(())
        };
        writer().map_err(|_| ParametersError::Fail.into_top())
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct Parameters {
    pub broker: BrokerParameters,
}

impl Export for Parameters {}

#[derive(Serialize, Deserialize, Default)]
pub struct ReplicaParameters {}

impl Export for ReplicaParameters {}

#[derive(Serialize, Deserialize)]
pub struct BrokerParameters {
    pub signup_batch_number: usize,
    pub signup_batch_size: usize,
}

impl Export for BrokerParameters {}

impl Default for BrokerParameters {
    fn default() -> Self {
        Self {
            signup_batch_number: 10,
            signup_batch_size: 5_000,
        }
    }
}
