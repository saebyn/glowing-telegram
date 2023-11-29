use serde::Deserialize;

fn deserialize_range<'de, D>(deserializer: D) -> Result<Option<(i64, i64)>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let s = String::deserialize(deserializer)?;

    let range = s
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|s| s.parse::<i64>())
        .collect::<Result<Vec<i64>, _>>()
        .map_err(|_| Error::custom("invalid range"))?;

    if range.len() != 2 {
        return Err(Error::custom("invalid range"));
    }

    Ok(Some((range[0], range[1])))
}

/**
 * Params for list endpoint for ra-data-simple-rest
 *
 * @see https://marmelab.com/react-admin/DataProviders.html#rest-api-parameters
 */
#[derive(Debug, Deserialize)]
pub struct ListParams {
    // Pagination
    // query string has the following format: range=[0, 24]
    // convert this string to a tuple
    #[serde(deserialize_with = "deserialize_range")]
    pub range: Option<(i64, i64)>,

    // Sort
    pub sort: Option<String>,
    pub order: Option<String>,
}
