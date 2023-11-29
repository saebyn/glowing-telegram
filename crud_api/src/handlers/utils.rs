use diesel::data_types::PgInterval;

pub fn parse_duration(duration: Option<String>) -> Result<PgInterval, String> {
    match duration
        .unwrap_or("P0S".to_string())
        .parse::<iso8601::Duration>()
    {
        Ok(durationValue) => Ok(PgInterval::from_microseconds(
            core::time::Duration::from(durationValue).as_micros() as i64,
        )),
        Err(e) => {
            tracing::error!("Error parsing duration: {}", e);
            Err(e.to_string())
        }
    }
}
