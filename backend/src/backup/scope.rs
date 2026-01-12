use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum BackupScope {
    User,
}

impl FromStr for BackupScope {
    type Err = serde_json::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct BackupScopeIndicator {
    pub scope: BackupScope,
    pub id: String,
    pub backup_id: String,
}

impl Serialize for BackupScopeIndicator {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let scope_string = match serde_json::to_string(&self.scope) {
            Ok(result) => result,
            Err(error) => return Err(serde::ser::Error::custom(error.to_string())),
        };

        let self_string: String = format!("{}/{}/{}", scope_string, self.id, self.backup_id);

        serializer.serialize_str(&self_string)
    }
}

impl<'de> Deserialize<'de> for BackupScopeIndicator {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        let parts: Vec<&str> = string.split('/').collect();

        if parts.len() != 3 {
            return Err(serde::de::Error::custom(
                "Invalid BackupScopeIndicator format",
            ));
        }

        let scope: BackupScope = match BackupScope::from_str(parts[0]) {
            Ok(result) => result,
            Err(error) => return Err(serde::de::Error::custom(error.to_string())),
        };

        Ok(Self {
            scope,
            id: parts[1].to_owned(),
            backup_id: parts[2].to_owned(),
        })
    }
}
