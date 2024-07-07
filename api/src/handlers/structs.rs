use serde::Deserialize;
use serde_json;

#[derive(Debug)]
pub struct Range {
    pub start: i64,
    pub count: i64,
}

fn deserialize_range<'de, D>(deserializer: D) -> Result<Range, D::Error>
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

    Ok(Range {
        start: range[0],
        count: range[1] - range[0] + 1,
    })
}

impl Default for Range {
    fn default() -> Self {
        Range {
            start: 0,
            count: 10,
        }
    }
}

// deserialize_sort takes a string like "[\"prefix\",\"DESC\"]" and returns a tuple
// like ("prefix", "DESC")
// it uses serde_json::Value to parse the string
fn deserialize_sort<'de, D>(deserializer: D) -> Result<Option<(String, String)>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let s = String::deserialize(deserializer)?;

    let sort =
        serde_json::from_str::<serde_json::Value>(&s).map_err(|_| Error::custom("invalid sort"))?;

    if sort.is_array() {
        let sort = sort.as_array().unwrap();

        if sort.len() != 2 {
            return Err(Error::custom("invalid sort"));
        }

        let sort = (
            sort[0].as_str().unwrap().to_string(),
            sort[1].as_str().unwrap().to_string(),
        );

        Ok(Some(sort))
    } else {
        Err(Error::custom("invalid sort"))
    }
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
    #[serde(deserialize_with = "deserialize_range", default)]
    pub range: Range,

    // Sort
    #[serde(deserialize_with = "deserialize_sort", default)]
    pub sort: Option<(String, String)>,

    // Filter
    pub filter: String,
}
