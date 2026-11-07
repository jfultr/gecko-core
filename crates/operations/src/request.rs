//! Start request, normalization for retry comparison, and parameter validation.
//!
//! The request carries no caller identity: the subject comes from the
//! [`CallerContext`](crate::CallerContext) the adapter supplies, and unknown
//! payload fields are rejected so a payload cannot smuggle one in.
//!
//! Retry comparison (model section 4, P4) uses the normalized operation type,
//! target, and typed parameters. Tracing metadata is excluded. Numbers are
//! compared by kind and value (Z4), and map key order does not matter.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};

use crate::api::{Rejection, RejectionCode, Violation};
use crate::identity::{CallerContext, RequestId, Subject};
use crate::value::{Parameters, ValueKind};

/// Domain operation kind, such as `test.echo`.
///
/// Non-empty; lowercase ASCII letters, digits, `.`, `_`, and `-` only.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct OperationType(String);

impl OperationType {
    /// Validates and creates an operation type.
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= 128
            && value
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b));
        if valid {
            Ok(Self(value))
        } else {
            Err(format!(
                "operation type `{value}` must be 1-128 characters of a-z, 0-9, `.`, `_`, `-`"
            ))
        }
    }

    /// Returns the type name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OperationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for OperationType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// The applicable target of an operation, such as a device or participant name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct Target(String);

impl Target {
    /// Creates a target, or an error message for an empty or whitespace-only name.
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        if value.trim().is_empty() {
            Err("target must not be empty".to_string())
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the target name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Target {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Tracing metadata. Never part of the retry comparison.
pub type TraceMetadata = BTreeMap<String, String>;

/// A Start request: transport-neutral input of the Start command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartRequest {
    /// Caller-supplied retry key within the caller's scope.
    pub request_id: RequestId,
    /// Operation kind.
    pub operation_type: OperationType,
    /// Applicable target, if the operation type has one.
    #[serde(default)]
    pub target: Option<Target>,
    /// Typed parameters.
    #[serde(default)]
    pub parameters: Parameters,
    /// Tracing metadata, excluded from retry comparison.
    #[serde(default)]
    pub trace: TraceMetadata,
}

impl StartRequest {
    /// Returns the normalized content compared on retry.
    pub fn fingerprint(&self) -> RequestFingerprint {
        RequestFingerprint {
            operation_type: self.operation_type.clone(),
            target: self.target.clone(),
            parameters: self.parameters.clone(),
        }
    }
}

/// Normalized request content: operation type, target, and typed parameters.
///
/// Excludes `request_id`, caller identity, and tracing metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestFingerprint {
    operation_type: OperationType,
    target: Option<Target>,
    parameters: Parameters,
}

/// The retry key: a request identifier scoped to a trusted caller subject.
///
/// The receiving Gecko is the outer scope (one Gecko owns one index). Two
/// callers using the same `request_id` never share a key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RetryKey {
    subject: Subject,
    request_id: RequestId,
}

impl RetryKey {
    /// Builds the key from the trusted caller context and the request identifier.
    pub fn new(caller: &CallerContext, request_id: RequestId) -> Self {
        Self {
            subject: caller.subject().clone(),
            request_id,
        }
    }

    /// Returns the subject part of the key.
    pub fn subject(&self) -> &Subject {
        &self.subject
    }

    /// Returns the request identifier part of the key.
    pub fn request_id(&self) -> RequestId {
        self.request_id
    }
}

/// Classification of a Start request against what is known for its retry key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryClass {
    /// No earlier request with this key: a new execution may be created.
    New,
    /// Same key and same content: return the earlier operation, create nothing.
    Replay,
    /// Same key, different content: reject as an identifier conflict.
    Conflict,
}

impl RetryClass {
    /// Compares `current` with the fingerprint recorded for the same key, if any.
    pub fn classify(previous: Option<&RequestFingerprint>, current: &RequestFingerprint) -> Self {
        match previous {
            None => Self::New,
            Some(previous) if previous == current => Self::Replay,
            Some(_) => Self::Conflict,
        }
    }
}

/// Whether an operation type takes a target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetRule {
    /// A target must not be given.
    Forbidden,
    /// A target may be given.
    Optional,
    /// A target must be given.
    Required,
}

/// Schema of one parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterSpec {
    /// Parameter name.
    pub name: String,
    /// Required value kind.
    pub kind: ValueKind,
    /// Whether the parameter must be present.
    pub required: bool,
}

/// Schema of one operation type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationSpec {
    /// The operation type described.
    pub operation_type: OperationType,
    /// Target rule.
    pub target: TargetRule,
    /// Accepted parameters; any other parameter is invalid.
    pub parameters: Vec<ParameterSpec>,
}

/// A request that passed validation against a [`Catalog`].
///
/// Only [`Catalog::validate`] creates one, so code that needs a validated
/// request cannot be handed an unvalidated one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedStart(StartRequest);

impl ValidatedStart {
    /// Returns the validated request.
    pub fn request(&self) -> &StartRequest {
        &self.0
    }
}

/// The operation types this Gecko accepts, with their parameter schemas.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    specs: BTreeMap<OperationType, OperationSpec>,
}

impl Catalog {
    /// Creates an empty catalog: every operation type is unknown.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an operation type, replacing an earlier spec for the same type.
    pub fn register(&mut self, spec: OperationSpec) -> &mut Self {
        self.specs.insert(spec.operation_type.clone(), spec);
        self
    }

    /// Validates `request`. An invalid request yields a [`Rejection`]; no
    /// operation exists for it.
    pub fn validate(&self, request: &StartRequest) -> Result<ValidatedStart, Rejection> {
        let Some(spec) = self.specs.get(&request.operation_type) else {
            return Err(Rejection::new(
                RejectionCode::UnknownOperationType,
                format!(
                    "operation type `{}` is not supported",
                    request.operation_type
                ),
                Vec::new(),
            ));
        };

        let mut violations = Vec::new();
        match (spec.target, &request.target) {
            (TargetRule::Required, None) => violations.push(Violation::new(
                "target",
                format!("operation type `{}` requires a target", spec.operation_type),
            )),
            (TargetRule::Forbidden, Some(_)) => violations.push(Violation::new(
                "target",
                format!("operation type `{}` takes no target", spec.operation_type),
            )),
            _ => {}
        }

        for (name, value) in &request.parameters {
            match spec.parameters.iter().find(|p| &p.name == name) {
                None => violations.push(Violation::new(
                    format!("parameters.{name}"),
                    "unknown parameter".to_string(),
                )),
                Some(p) if p.kind != value.kind() => violations.push(Violation::new(
                    format!("parameters.{name}"),
                    format!("expected {}, got {}", p.kind, value.kind()),
                )),
                Some(_) => {}
            }
        }
        for p in spec.parameters.iter().filter(|p| p.required) {
            if !request.parameters.contains_key(&p.name) {
                violations.push(Violation::new(
                    format!("parameters.{}", p.name),
                    "required parameter is missing".to_string(),
                ));
            }
        }

        if violations.is_empty() {
            Ok(ValidatedStart(request.clone()))
        } else {
            Err(Rejection::new(
                RejectionCode::InvalidParameters,
                format!(
                    "request for `{}` has {} invalid field(s)",
                    spec.operation_type,
                    violations.len()
                ),
                violations,
            ))
        }
    }
}
