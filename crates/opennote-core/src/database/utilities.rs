use chrono::NaiveDateTime;

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
        Err(_) => {
            0
        }
    }
}
