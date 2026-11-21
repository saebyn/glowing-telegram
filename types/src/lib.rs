mod types;
pub mod utils;

pub use types::*;

/// An optional field may be omitted, but a present value must not be null.
pub fn deserialize_present<'de, D, T>(
    deserializer: D,
) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}
