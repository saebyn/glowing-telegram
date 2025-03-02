use crate::{CutList, CutListClass, Episode};
use aws_sdk_dynamodb::types::AttributeValue;
use std::{collections::HashMap, convert::From};

impl From<CutListClass> for CutList {
    fn from(cut_list: CutListClass) -> Self {
        CutList {
            input_media: cut_list.input_media,
            output_track: cut_list.output_track,
            overlay_tracks: cut_list.overlay_tracks,
            version: cut_list.version,
        }
    }
}

impl TryFrom<HashMap<String, AttributeValue>> for Episode {
    type Error = serde_json::Error;

    fn try_from(
        item: HashMap<String, AttributeValue>,
    ) -> Result<Self, Self::Error> {
        let value = convert_hm_to_json(item);

        serde_json::from_value(value)
    }
}

// Convert a hashmap with `AttributeValue`s to a JSON object
//
// # Arguments
//
// * `hm` - The hashmap to convert.
//
// # Returns
//
// A `serde_json::Value` representing the converted hashmap.
//
// # Example
//
// ```rust
// let hm = hashmap! {
//     "id".to_string() => AttributeValue::S("123".to_string()),
//     "name".to_string() => AttributeValue::S("John Doe".to_string()),
// };
//
// let json = convert_hm_to_json(hm);
// ```
//
// The `json` variable will contain the following JSON object:
//
// ```json
// {
//     "id": "123",
//     "name": "John Doe"
// }
// ```
pub fn convert_hm_to_json(
    hm: HashMap<String, AttributeValue>,
) -> serde_json::Value {
    hm.into_iter()
        .map(|(k, v)| (k, convert_attribute_value_to_json(v)))
        .collect()
}

pub fn convert_json_to_hm(
    json: &serde_json::Value,
) -> HashMap<String, AttributeValue> {
    json.as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), convert_json_to_attribute_value(v.clone())))
        .collect()
}

/// Converts a ``DynamoDB`` attribute value to a JSON value.
///
/// # Arguments
///
/// * `attribute_value` - The ``DynamoDB`` attribute value to convert.
///
/// # Returns
///
/// A `serde_json::Value` representing the converted attribute value.
pub fn convert_attribute_value_to_json(
    attribute_value: AttributeValue,
) -> serde_json::Value {
    match attribute_value {
        AttributeValue::S(s) => serde_json::Value::String(s),
        AttributeValue::N(n) => serde_json::Value::Number(
            serde_json::Number::from_f64(n.parse().unwrap()).unwrap(),
        ),
        AttributeValue::Bool(b) => serde_json::Value::Bool(b),
        AttributeValue::L(l) => serde_json::Value::Array(
            l.into_iter().map(convert_attribute_value_to_json).collect(),
        ),
        AttributeValue::M(m) => serde_json::Value::Object(
            m.into_iter()
                .map(|(k, v)| (k, convert_attribute_value_to_json(v)))
                .collect(),
        ),
        AttributeValue::Ss(ss) => serde_json::Value::Array(
            ss.into_iter().map(serde_json::Value::String).collect(),
        ),
        AttributeValue::Ns(ns) => serde_json::Value::Array(
            ns.into_iter()
                .map(|n| {
                    serde_json::Value::Number(
                        serde_json::Number::from_f64(n.parse().unwrap())
                            .unwrap(),
                    )
                })
                .collect(),
        ),
        _ => serde_json::Value::Null,
    }
}

pub fn convert_json_to_attribute_value(
    json: serde_json::Value,
) -> AttributeValue {
    match json {
        serde_json::Value::String(s) => AttributeValue::S(s),
        serde_json::Value::Number(n) => AttributeValue::N(n.to_string()),
        serde_json::Value::Bool(b) => AttributeValue::Bool(b),
        serde_json::Value::Array(a) => AttributeValue::L(
            a.into_iter().map(convert_json_to_attribute_value).collect(),
        ),
        serde_json::Value::Object(o) => AttributeValue::M(
            o.into_iter()
                .map(|(k, v)| (k, convert_json_to_attribute_value(v)))
                .collect(),
        ),
        serde_json::Value::Null => AttributeValue::Null(true),
    }
}
