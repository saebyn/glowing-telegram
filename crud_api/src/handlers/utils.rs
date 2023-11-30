use diesel::data_types::PgInterval;

pub fn parse_duration(duration: Option<String>) -> Result<PgInterval, String> {
    match duration
        .unwrap_or("PT0S".to_string())
        .parse::<iso8601::Duration>()
    {
        Ok(duration_value) => Ok(PgInterval::from_microseconds(
            core::time::Duration::from(duration_value).as_micros() as i64,
        )),
        Err(e) => {
            tracing::error!("Error parsing duration: {}", e);
            Err(e.to_string())
        }
    }
}
