use uuid::Uuid;
use bs58::{decode, encode};
use std::path::Path;
use crate::model::{LiplError, LiplResult};

pub trait UuidExt {
    fn to_base58(&self) -> String;
    fn from_base58<P: AsRef<Path>>(p: P) -> Uuid;
    fn try_from_base58<P: AsRef<Path>>(p: P) -> LiplResult<Uuid>;
}

impl UuidExt for Uuid {
    fn to_base58(&self) -> String {
        let bytes = self.as_bytes();
        encode(bytes).into_string()
    }

    fn from_base58<P: AsRef<Path>>(p: P) -> Self {
        Uuid::try_from_base58(p).unwrap()
    }

    fn try_from_base58<P: AsRef<Path>>(p: P) -> LiplResult<Uuid> {
        let mut decoded = [0xFF; 16];

        decode(
            p.as_ref().file_stem().ok_or(LiplError::NoPath("".to_owned()))?.to_string_lossy().to_string().as_str()
        )
        .into(&mut decoded)?;

        let uuid = Uuid::from_slice(&decoded)?;
        Ok(uuid)
    }
}
