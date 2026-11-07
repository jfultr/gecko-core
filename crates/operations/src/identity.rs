//! Request and execution identities and the trusted caller context.
//!
//! Both identifiers are random UUIDv4 values in the first experiment (model
//! section 4, P4). The caller supplies the [`RequestId`]; the receiving Gecko
//! assigns the [`OperationId`] on acceptance. The caller identity is never a
//! request field: it is a [`CallerContext`] constructed by the transport
//! adapter from its trusted configuration.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};
use uuid::{Uuid, Version};

/// Error returned when text is not a valid UUIDv4 identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidIdentifier {
    /// The rejected input.
    pub input: String,
}

impl fmt::Display for InvalidIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}` is not a UUIDv4 identifier", self.input)
    }
}

impl std::error::Error for InvalidIdentifier {}

fn parse_v4(text: &str) -> Result<Uuid, InvalidIdentifier> {
    Uuid::parse_str(text)
        .ok()
        .filter(|uuid| uuid.get_version() == Some(Version::Random))
        .ok_or_else(|| InvalidIdentifier {
            input: text.to_string(),
        })
}

macro_rules! uuid_v4_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Generates a new random UUIDv4 identifier.
            pub fn generate() -> Self {
                Self(Uuid::new_v4())
            }

            /// Parses a UUIDv4 in its textual form; other UUID versions are rejected.
            pub fn parse(text: &str) -> Result<Self, InvalidIdentifier> {
                parse_v4(text).map(Self)
            }

            /// Returns the underlying UUID.
            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.hyphenated().fmt(f)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let text = String::deserialize(deserializer)?;
                Self::parse(&text).map_err(serde::de::Error::custom)
            }
        }
    };
}

uuid_v4_id! {
    /// Caller-supplied identifier of one Start request; the retry key within a caller scope.
    ///
    /// Differs from [`OperationId`], which the receiving Gecko assigns to the execution.
    RequestId
}

uuid_v4_id! {
    /// Identifier of one operation execution, assigned by the receiving Gecko on acceptance.
    OperationId
}

/// The authenticated subject on whose behalf a command is issued.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Subject(String);

impl Subject {
    /// Creates a subject, or `None` for an empty or whitespace-only name.
    pub fn new(name: impl Into<String>) -> Option<Self> {
        let name = name.into();
        (!name.trim().is_empty()).then_some(Self(name))
    }

    /// Returns the subject name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Trusted caller context supplied by the transport adapter.
///
/// This type is deliberately neither `Serialize` nor `Deserialize`: it cannot
/// be read from a request payload. The adapter builds it from its trusted
/// configuration (restricted `0.1.0` test deployment) or, later, from
/// authenticated transport identity. Core only consumes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallerContext {
    subject: Subject,
    adapter: String,
}

impl CallerContext {
    /// Creates a context for `subject`, recording which adapter vouched for it.
    pub fn from_adapter(adapter: impl Into<String>, subject: Subject) -> Self {
        Self {
            subject,
            adapter: adapter.into(),
        }
    }

    /// Returns the trusted subject.
    pub fn subject(&self) -> &Subject {
        &self.subject
    }

    /// Returns the name of the adapter that supplied this context.
    pub fn adapter(&self) -> &str {
        &self.adapter
    }
}
