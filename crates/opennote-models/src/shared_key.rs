use anyhow::Result;
use serde::{Deserialize, Deserializer, Serializer};
use serde_encrypt::{AsSharedKey, shared_key::SharedKey};

use crate::constants::hex::{HEX_DIGITS, HEX_LENGTH, KEY_LENGTH};

#[derive(Deserialize)]
#[serde(untagged)]
enum SharedKeyRepresentation {
    Hex(String),
    // Backward compatibility with the existing JSON representation.
    Bytes([u8; 32]),
}

pub(crate) fn serialize<S>(key: &SharedKey, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut encoded = String::with_capacity(HEX_LENGTH);

    for byte in key.as_slice() {
        encoded.push(HEX_DIGITS[(byte >> 4) as usize] as char);
        encoded.push(HEX_DIGITS[(byte & 0x0f) as usize] as char);
    }

    serializer.serialize_str(&encoded)
}

pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<SharedKey, D::Error>
where
    D: Deserializer<'de>,
{
    match SharedKeyRepresentation::deserialize(deserializer)? {
        SharedKeyRepresentation::Hex(encoded) => {
            Ok(decode_hex(&encoded).map(SharedKey::new).unwrap())
        }
        SharedKeyRepresentation::Bytes(bytes) => Ok(SharedKey::new(bytes)),
    }
}

fn decode_hex(encoded: &str) -> Result<[u8; KEY_LENGTH], String> {
    if encoded.len() != HEX_LENGTH {
        return Err(format!(
            "shared key must contain exactly {HEX_LENGTH} hexadecimal characters"
        ));
    }

    let mut decoded = [0; KEY_LENGTH];

    for (index, pair) in encoded.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let high = decode_nibble(pair[0])
            .ok_or_else(|| format!("invalid hexadecimal character at position {}", index * 2))?;
        let low = decode_nibble(pair[1]).ok_or_else(|| {
            format!(
                "invalid hexadecimal character at position {}",
                index * 2 + 1
            )
        })?;

        decoded[index] = (high << 4) | low;
    }

    Ok(decoded)
}

fn decode_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}
