use doomstack::{Doom, Top};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{BufWriter, Write as _},
};

use crate::executables::replica::ReplicaError;

pub trait Export: Serialize + DeserializeOwned {
    fn read(path: &str) -> Result<Self, Top<ReplicaError>> {
        let reader = || -> Result<Self, std::io::Error> {
            let data = fs::read(path)?;
            Ok(serde_json::from_slice(data.as_slice())?)
        };
        reader().map_err(|_| ReplicaError::Fail.into_top())
    }

    fn write(&self, path: &str) -> Result<(), Top<ReplicaError>> {
        let writer = || -> Result<(), std::io::Error> {
            let file = OpenOptions::new().create(true).write(true).open(path)?;
            let mut writer = BufWriter::new(file);
            let data = serde_json::to_string_pretty(self).unwrap();
            writer.write_all(data.as_ref())?;
            writer.write_all(b"\n")?;
            Ok(())
        };
        writer().map_err(|_| ReplicaError::Fail.into_top())
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct Parameters {}

impl Export for Parameters {}
