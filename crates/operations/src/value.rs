//! Typed structured values used for request parameters and report content.
//!
//! Numbers keep their kind: an integer and a float are different values, so
//! `1` and `1.0` never compare equal (decision Z4: retry comparison is
//! spelling-sensitive in `0.1.0`). Floats must be finite so that equality is
//! total and the retry comparison is well defined. Maps are ordered by key, so
//! key order in a payload never affects comparison.

use std::collections::BTreeMap;
use std::fmt;

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

/// A finite 64-bit float with total equality (compared by bit pattern).
///
/// `0.0` and `-0.0` are different values, consistent with spelling-sensitive
/// comparison.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(transparent)]
pub struct FiniteF64(f64);

impl FiniteF64 {
    /// Wraps `value`, or returns `None` for NaN and infinities.
    pub fn new(value: f64) -> Option<Self> {
        value.is_finite().then_some(Self(value))
    }

    /// Returns the wrapped number.
    pub fn get(self) -> f64 {
        self.0
    }
}

impl PartialEq for FiniteF64 {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for FiniteF64 {}

impl<'de> Deserialize<'de> for FiniteF64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = f64::deserialize(deserializer)?;
        Self::new(value).ok_or_else(|| serde::de::Error::custom("float must be finite"))
    }
}

/// A typed structured value.
///
/// Deserialization is strict: an integer outside the `i64` range is an error
/// instead of silently becoming a float.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Value {
    /// Absence of a value.
    Null,
    /// A boolean.
    Bool(bool),
    /// A signed integer. Distinct from [`Value::Float`] even for equal magnitudes.
    Integer(i64),
    /// A finite float.
    Float(FiniteF64),
    /// A text string.
    Text(String),
    /// An ordered list.
    List(Vec<Value>),
    /// A map ordered by key.
    Map(BTreeMap<String, Value>),
}

impl Value {
    /// Creates a float value, or `None` when `value` is not finite.
    pub fn float(value: f64) -> Option<Self> {
        FiniteF64::new(value).map(Self::Float)
    }

    /// Returns the kind of this value.
    pub fn kind(&self) -> ValueKind {
        match self {
            Self::Null => ValueKind::Null,
            Self::Bool(_) => ValueKind::Bool,
            Self::Integer(_) => ValueKind::Integer,
            Self::Float(_) => ValueKind::Float,
            Self::Text(_) => ValueKind::Text,
            Self::List(_) => ValueKind::List,
            Self::Map(_) => ValueKind::Map,
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a null, bool, i64 integer, finite float, string, list, or map")
            }

            fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            fn visit_none<E: de::Error>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
                Value::deserialize(d)
            }

            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Value, E> {
                Ok(Value::Bool(v))
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Value, E> {
                Ok(Value::Integer(v))
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Value, E> {
                i64::try_from(v)
                    .map(Value::Integer)
                    .map_err(|_| E::custom("integer is outside the i64 range"))
            }

            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Value, E> {
                Value::float(v).ok_or_else(|| E::custom("float must be finite"))
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Value, E> {
                Ok(Value::Text(v.to_string()))
            }

            fn visit_string<E: de::Error>(self, v: String) -> Result<Value, E> {
                Ok(Value::Text(v))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element()? {
                    items.push(item);
                }
                Ok(Value::List(items))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
                let mut entries = BTreeMap::new();
                while let Some((key, value)) = map.next_entry::<String, Value>()? {
                    entries.insert(key, value);
                }
                Ok(Value::Map(entries))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

/// The kind of a [`Value`], used by parameter schemas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueKind {
    /// [`Value::Null`].
    Null,
    /// [`Value::Bool`].
    Bool,
    /// [`Value::Integer`].
    Integer,
    /// [`Value::Float`].
    Float,
    /// [`Value::Text`].
    Text,
    /// [`Value::List`].
    List,
    /// [`Value::Map`].
    Map,
}

impl fmt::Display for ValueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Null => "null",
            Self::Bool => "bool",
            Self::Integer => "integer",
            Self::Float => "float",
            Self::Text => "text",
            Self::List => "list",
            Self::Map => "map",
        })
    }
}

/// Typed request parameters, ordered by name.
pub type Parameters = BTreeMap<String, Value>;
