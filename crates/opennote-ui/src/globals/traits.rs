use anyhow::Result;
use serde::{Serialize, de::DeserializeOwned};

pub trait Load
where
    Self: Sized + Serialize + DeserializeOwned,
{
    fn load() -> Result<Self>;
}
