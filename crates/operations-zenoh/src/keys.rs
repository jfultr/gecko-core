//! Key expressions of the Operation Zenoh mapping.

use operations::{ObservationAddress, OperationId};

/// Default key prefix of the first experiment. Provisional, not a contract.
pub const DEFAULT_PREFIX: &str = "gecko/g1/op";

/// Key expressions under one prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keys {
    prefix: String,
}

impl Default for Keys {
    fn default() -> Self {
        Self::new(DEFAULT_PREFIX)
    }
}

impl Keys {
    /// Keys under `prefix`, without a trailing slash.
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into().trim_end_matches('/').to_string(),
        }
    }

    /// Where Start requests are sent.
    pub fn start(&self) -> String {
        format!("{}/start", self.prefix)
    }

    /// Where the latest report of `id` is read.
    pub fn report(&self, id: OperationId) -> String {
        format!("{}/{id}/report", self.prefix)
    }

    /// Where `id` is observed.
    pub fn observe(&self, id: OperationId) -> String {
        format!("{}/{id}/observe", self.prefix)
    }

    /// The observation address Start returns for `id`; give it to
    /// [`Acceptor::with_observation_address`](operations::Acceptor::with_observation_address).
    pub fn address(&self, id: OperationId) -> ObservationAddress {
        ObservationAddress(self.observe(id))
    }

    pub(crate) fn report_pattern(&self) -> String {
        format!("{}/*/report", self.prefix)
    }

    pub(crate) fn observe_pattern(&self) -> String {
        format!("{}/*/observe", self.prefix)
    }

    /// The operation id in a `<prefix>/<id>/<action>` key, if it parses as one.
    pub(crate) fn operation_id(&self, key: &str) -> Option<OperationId> {
        let rest = key.strip_prefix(&self.prefix)?.strip_prefix('/')?;
        OperationId::parse(rest.split('/').next()?).ok()
    }
}
