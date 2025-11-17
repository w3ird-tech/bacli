use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringOrInt {
    String(String),
    Int(i64),
}

impl StringOrInt {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            StringOrInt::String(s) => Some(s),
            StringOrInt::Int(_) => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            StringOrInt::String(_) => None,
            StringOrInt::Int(i) => Some(*i),
        }
    }

    pub fn into_string(self) -> String {
        match self {
            StringOrInt::String(s) => s,
            StringOrInt::Int(i) => i.to_string(),
        }
    }

    pub fn into_i64(self) -> Option<i64> {
        match self {
            StringOrInt::String(s) => s.parse().ok(),
            StringOrInt::Int(i) => Some(i),
        }
    }
}

#[cfg(test)]
mod tests {
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
}
