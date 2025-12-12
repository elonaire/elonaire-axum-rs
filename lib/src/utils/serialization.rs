use serde::{Deserialize, Deserializer, Serializer};
use std::fmt::Display;
use std::str::FromStr;

// Generic serialization function for f32 or f64
pub fn serialize_float<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Display + PartialEq + Copy, // Traits for formatting and checking special cases
    f64: From<T>,                  // Ensure T can be converted to f64 for is_nan/is_infinite
    S: Serializer,
{
    let as_f64: f64 = (*value).into(); // Convert to f64 for checking special cases
    let str_value = convert_float_to_string(as_f64);

    serializer.serialize_str(&str_value)
}

// Generic deserialization function for f32 or f64
pub fn deserialize_float<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,      // Trait for parsing strings
    T::Err: Display, // Ensure parsing errors can be displayed
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(|e| serde::de::Error::custom(format!("Failed to parse float: {}", e)))
}

pub fn convert_float_to_string(value: f64) -> String {
    let as_f64 = value;
    if as_f64.is_nan() {
        "NaN".into()
    } else if as_f64.is_infinite() {
        if as_f64 > 0.0 {
            "Infinity"
        } else {
            "-Infinity"
        }
        .into()
    } else {
        // Format with limited precision (e.g., 6 decimal places)
        format!("{:.6}", value)
    }
}
