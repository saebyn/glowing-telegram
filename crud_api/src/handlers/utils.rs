use diesel::data_types::PgInterval;

pub fn parse_duration(duration: Option<String>) -> PgInterval {
    match duration
        .unwrap_or("PT0S".to_string())
        .parse::<iso8601::Duration>()
    {
        Ok(duration_value) => PgInterval::from_microseconds(
            core::time::Duration::from(duration_value).as_micros() as i64,
        ),
        Err(e) => {
            tracing::error!("Error parsing duration: {}", e);
            PgInterval::from_microseconds(0)
        }
    }
}

pub fn parse_duration_to_string(duration: PgInterval) -> String {
    chrono::Duration::microseconds(duration.microseconds).to_string()
}
