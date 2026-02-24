use std::collections::HashMap;

use chrono::NaiveDateTime;

use crate::documents::traits::GetId;

pub fn parse_timestamp(s: &str) -> i64 {
    // The format: "%Y-%m-%d %H:%M:%S%.f +00"
    // - %Y : 4‑digit year
    // - %m : 2‑digit month
    // - %d : 2‑digit day
    // - %H : hour (1–2 digits)
    // - %M : minute (2 digits)
    // - %S : second (2 digits)
    // - %.f : optional fractional seconds (any number of digits)
    // - " +00" : literal space and "+00"
    match NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f +00") {
        Ok(result) => result.and_utc().timestamp(),
        Err(error) => {
            log::warn!(
                "Failed to parse timestamp '{}', defaulting to 0, error: {}",
                s,
                error
            );
            0
        }
    }
}

/// Re-order a list of data into the order specified by the ids vector.
pub fn map_order_by_ids<T: Into<S>, S: GetId>(data: Vec<T>, ids: &Vec<String>) -> Vec<S> {
    let mut id_map: HashMap<String, S> = data.into_iter()
        .map(|item| {
            let metadata: S = item.into();
            (metadata.get_id().to_string(), metadata)
        })
        .collect();

    ids.iter().filter_map(|item| id_map.remove(item)).collect()
}
