use serde::{Deserializer, Serializer};
use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
#[serde(tag = "type", content = "pattern")]
pub enum Pattern {
    #[serde(rename = "wildcard")]
    Wildcard(String),
    #[serde(rename = "match")]
    Match(String),
    #[serde(rename = "regex")]
    Regex(String),
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub struct Rule {
    title: String,
    #[serde(flatten)]
    pattern: Pattern,
    active: bool,
    #[serde(serialize_with = "ser_include")]
    #[serde(deserialize_with = "de_include")]
    include: bool,
}

impl Rule {
    pub fn new<S>(title: S, pattern: Pattern, active: bool, include: bool) -> Self
    where
        S: Into<String>,
    {
        Self {
            title: title.into(),
            pattern,
            active,
            include,
        }
    }
}

fn ser_include<S>(val: &bool, ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if *val {
        ser.serialize_str("include")
    } else {
        ser.serialize_str("exclude")
    }
}

fn de_include<'de, D>(de: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    match <&str as serde::Deserialize>::deserialize(de)? {
        "include" => Ok(true),
        "exclude" => Ok(false),
        s => Err(serde::de::Error::unknown_variant(s, &[])),
    }
}

#[cfg(test)]
mod tests {
    use crate::foxyproxy::{Pattern, Rule};

    #[test]
    fn test_serde_json() {
        let rule = Rule {
            title: "test".to_string(),
            pattern: Pattern::Wildcard("*://*.csdn.com/".to_string()),
            active: true,
            include: true,
        };

        let json = serde_json::to_string(&rule).expect("failed to serialize rule");

        let de_rule = serde_json::from_str::<Rule>(&json).expect("failed to deserialize rule");

        assert_eq!(rule, de_rule);
    }
}
