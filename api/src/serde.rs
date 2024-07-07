use serde::Deserialize;

// take a string representing an ISO8601 duration and convert it to std::time::Duration, using the iso8601 crate
pub fn deserialize_duration<'de, D>(deserializer: D) -> Result<std::time::Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    let duration = s
        .parse::<iso8601::Duration>()
        .map_err(serde::de::Error::custom)?;
    Ok(duration.into())
}

// take a std::time::Duration and convert it to a string representing an ISO8601 duration, using the chrono crate
pub fn serialize_duration<S>(
    duration: &std::time::Duration,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let duration = chrono::Duration::from_std(*duration).map_err(serde::ser::Error::custom)?;
    let s = format!("{}", duration);
    serializer.serialize_str(&s)
}
