use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct RemoteServerConfiguration {
    pub connection_string: String,
    pub password: String,
}
