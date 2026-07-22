use serde::{Deserialize, Serialize};
use serde_encrypt::shared_key::SharedKey;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteServerConfiguration {
    /// The base url of the server
    pub connection_string: String,

    /// This must be the same as the one set on the server side
    pub password: String,

    /// This must be the same as the one set on the server side
    pub shared_key: SharedKey,
}
