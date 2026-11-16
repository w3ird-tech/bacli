use std::fmt;
use std::marker::PhantomData;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{DeserializeAs, SerializeAs};

/// A value the firmware has reported as a string in some versions and as a number in others.
///
/// For example, `bestDiff` was a formatted string (`"4.29G"`) before ESP-Miner v2.11.0 and a
/// plain number afterwards.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum StringOrInt {
    String(String),
    Int(i64),
    Float(f64),
}

impl StringOrInt {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            StringOrInt::String(s) => Some(s),
            StringOrInt::Int(_) | StringOrInt::Float(_) => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            StringOrInt::String(_) | StringOrInt::Float(_) => None,
            StringOrInt::Int(i) => Some(*i),
        }
    }

    pub fn into_string(self) -> String {
        match self {
            StringOrInt::String(s) => s,
            StringOrInt::Int(i) => i.to_string(),
            StringOrInt::Float(f) => f.to_string(),
        }
    }

    pub fn into_i64(self) -> Option<i64> {
        match self {
            StringOrInt::String(s) => s.parse().ok(),
            StringOrInt::Int(i) => Some(i),
            StringOrInt::Float(f) => float_to_int(f),
        }
    }
}

/// Deserializes an integer from any JSON number.
///
/// Newer firmware writes some integer settings as floats (`525.0`), and older firmware writes
/// some float measurements that happen to be whole numbers as integers. Fractional values are
/// rounded to the nearest integer.
pub(crate) struct LenientInt;

impl<'de, T> DeserializeAs<'de, T> for LenientInt
where
    T: TryFrom<i128>,
{
    fn deserialize_as<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(LenientIntVisitor(PhantomData))
    }
}

impl<T> SerializeAs<T> for LenientInt
where
    T: Serialize,
{
    fn serialize_as<S>(source: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        source.serialize(serializer)
    }
}

struct LenientIntVisitor<T>(PhantomData<T>);

impl<T> LenientIntVisitor<T>
where
    T: TryFrom<i128>,
{
    fn convert<E: de::Error>(value: i128) -> Result<T, E> {
        T::try_from(value).map_err(|_| E::custom(format!("integer {value} is out of range")))
    }
}

impl<T> Visitor<'_> for LenientIntVisitor<T>
where
    T: TryFrom<i128>,
{
    type Value = T;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a number")
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<T, E> {
        Self::convert(v.into())
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<T, E> {
        Self::convert(v.into())
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<T, E> {
        if !v.is_finite() {
            return Err(E::invalid_value(de::Unexpected::Float(v), &self));
        }

        Self::convert(v.round() as i128)
    }
}

/// Deserializes a boolean from a JSON boolean or number (`0` is false, anything else is true).
///
/// The firmware reports most flags as `0`/`1`, but newer versions use real booleans for some.
pub(crate) struct LenientBool;

impl<'de> DeserializeAs<'de, bool> for LenientBool {
    fn deserialize_as<D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(LenientBoolVisitor)
    }
}

impl SerializeAs<bool> for LenientBool {
    fn serialize_as<S>(source: &bool, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(*source)
    }
}

struct LenientBoolVisitor;

impl Visitor<'_> for LenientBoolVisitor {
    type Value = bool;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a boolean or a number")
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> Result<bool, E> {
        Ok(v)
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<bool, E> {
        Ok(v != 0)
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<bool, E> {
        Ok(v != 0)
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<bool, E> {
        Ok(v != 0.0)
    }
}

fn float_to_int(value: f64) -> Option<i64> {
    let rounded = value.round();
    (rounded.is_finite() && rounded >= i64::MIN as f64 && rounded <= i64::MAX as f64)
        .then_some(rounded as i64)
}

#[cfg(test)]
mod tests {
    use serde_with::serde_as;

    use super::*;

    #[test]
    fn test_deserialize_from_string() {
        let json = r#""hello""#;
        let result: StringOrInt = serde_json::from_str(json).unwrap();
        assert!(matches!(result, StringOrInt::String(_)));
        assert_eq!(result.as_str(), Some("hello"));
    }

    #[test]
    fn test_deserialize_from_int() {
        let json = r#"42"#;
        let result: StringOrInt = serde_json::from_str(json).unwrap();
        assert!(matches!(result, StringOrInt::Int(_)));
        assert_eq!(result.as_int(), Some(42));
    }

    #[test]
    fn test_deserialize_from_float() {
        let result: StringOrInt = serde_json::from_str("1.5e15").unwrap();
        assert_eq!(result, StringOrInt::Float(1.5e15));
        assert_eq!(result.into_i64(), Some(1_500_000_000_000_000));
    }

    #[test]
    fn test_serialize_string() {
        let value = StringOrInt::String("test".to_string());
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, "\"test\"");
    }

    #[test]
    fn test_serialize_int() {
        let value = StringOrInt::Int(123);
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, "123");
    }

    #[test]
    fn test_into_string() {
        let s = StringOrInt::String("world".to_string()).into_string();
        assert_eq!(s, "world");

        let i = StringOrInt::Int(456).into_string();
        assert_eq!(i, "456");
    }

    #[test]
    fn test_into_i64() {
        let s = StringOrInt::String("789".to_string()).into_i64();
        assert_eq!(s, Some(789));

        let i = StringOrInt::Int(999).into_i64();
        assert_eq!(i, Some(999));

        let invalid = StringOrInt::String("not_a_number".to_string()).into_i64();
        assert_eq!(invalid, None);
    }

    #[serde_as]
    #[derive(Debug, Deserialize)]
    struct Lenient {
        #[serde_as(as = "LenientInt")]
        unsigned: u16,
        #[serde_as(as = "LenientInt")]
        signed: i64,
        #[serde_as(as = "LenientBool")]
        flag: bool,
    }

    #[test]
    fn lenient_values_accept_ints_floats_and_bools() {
        let from_ints: Lenient =
            serde_json::from_value(serde_json::json!({"unsigned": 525, "signed": -1, "flag": 1}))
                .unwrap();
        assert_eq!(from_ints.unsigned, 525);
        assert_eq!(from_ints.signed, -1);
        assert!(from_ints.flag);

        let from_floats: Lenient = serde_json::from_value(
            serde_json::json!({"unsigned": 525.0, "signed": -1.4, "flag": false}),
        )
        .unwrap();
        assert_eq!(from_floats.unsigned, 525);
        assert_eq!(from_floats.signed, -1);
        assert!(!from_floats.flag);
    }

    #[test]
    fn lenient_int_rejects_out_of_range_values() {
        let result: Result<Lenient, _> =
            serde_json::from_value(serde_json::json!({"unsigned": -1, "signed": 0, "flag": 0}));
        assert!(result.is_err());
    }
}
