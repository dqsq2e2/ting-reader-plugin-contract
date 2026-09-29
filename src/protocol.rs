//! Common control-message result shape for all nine capabilities and runtimes.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginErrorCode {
    InvalidManifest,
    InvalidInput,
    InvalidOutput,
    UnsupportedOperation,
    PermissionDenied,
    NotFound,
    Conflict,
    NetworkError,
    ParseError,
    Timeout,
    Cancelled,
    ResourceLimit,
    DependencyError,
    AbiMismatch,
    PluginUnavailable,
    InternalError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginError {
    pub code: PluginErrorCode,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub plugin_id: String,
    pub capability_id: String,
    pub operation: String,
    pub request_id: String,
    pub retryable: bool,
}

impl PluginError {
    /// This validates the shape only; the Host still redacts details, binds
    /// identity to the call context and enforces its control-message budget.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.message.is_empty() || self.message.len() > 512 {
            return Err("error message must contain 1..=512 UTF-8 bytes");
        }
        if [
            &self.plugin_id,
            &self.capability_id,
            &self.operation,
            &self.request_id,
        ]
        .into_iter()
        .any(|id| id.is_empty() || id.len() > 128)
        {
            return Err("error identity fields must contain 1..=128 bytes");
        }
        if self
            .details
            .as_ref()
            .is_some_and(|value| serde_json::to_vec(value).is_ok_and(|bytes| bytes.len() > 4096))
        {
            return Err("error details exceed 4096 bytes");
        }
        Ok(())
    }

    /// Gateway-owned mapping; plugins cannot return arbitrary HTTP statuses.
    /// The gateway decides whether a limit was imposed on the client request
    /// or during plugin execution.
    pub fn http_status(&self, client_limit: bool) -> u16 {
        match self.code {
            PluginErrorCode::InvalidInput => 400,
            PluginErrorCode::PermissionDenied => 403,
            PluginErrorCode::NotFound => 404,
            PluginErrorCode::Conflict => 409,
            PluginErrorCode::PluginUnavailable | PluginErrorCode::DependencyError => 503,
            PluginErrorCode::Timeout => 504,
            PluginErrorCode::ResourceLimit if client_limit => 429,
            _ => 502,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct True;

impl Serialize for True {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bool(true)
    }
}

impl<'de> Deserialize<'de> for True {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if bool::deserialize(deserializer)? {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom("expected true"))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct False;

impl Serialize for False {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bool(false)
    }
}

impl<'de> Deserialize<'de> for False {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if !bool::deserialize(deserializer)? {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom("expected false"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallSuccess<T> {
    pub ok: True,
    pub data: T,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallFailure {
    pub ok: False,
    pub error: PluginError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CallResult<T> {
    Success(CallSuccess<T>),
    Failure(CallFailure),
}

impl<T> CallResult<T> {
    pub fn success(data: T) -> Self {
        Self::Success(CallSuccess { ok: True, data })
    }

    pub fn failure(error: PluginError) -> Self {
        Self::Failure(CallFailure { ok: False, error })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, to_value};

    fn sample_error() -> PluginError {
        PluginError {
            code: PluginErrorCode::InvalidInput,
            message: "invalid value".into(),
            details: None,
            plugin_id: "provider".into(),
            capability_id: "format.handler".into(),
            operation: "probe".into(),
            request_id: "req-1".into(),
            retryable: false,
        }
    }

    #[test]
    fn success_and_failure_have_exclusive_checked_envelopes() {
        let success = CallResult::success(json!({"value": 1}));
        assert_eq!(
            to_value(&success).unwrap(),
            json!({"ok": true, "data": {"value": 1}})
        );
        let failure: CallResult<serde_json::Value> = CallResult::failure(sample_error());
        let encoded = to_value(&failure).unwrap();
        assert_eq!(encoded["ok"], false);
        assert_eq!(encoded["error"]["code"], "invalid_input");
        assert!(
            serde_json::from_value::<CallResult<serde_json::Value>>(json!({
                "ok": true, "error": to_value(sample_error()).unwrap()
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<CallResult<serde_json::Value>>(json!({
                "ok": false, "data": {}
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<CallResult<serde_json::Value>>(json!({
                "ok": true, "data": {}, "extra": "ignored"
            }))
            .is_err()
        );
    }

    #[test]
    fn gateway_http_mapping_and_error_budget() {
        let mut error = sample_error();
        error.validate().unwrap();
        assert_eq!(error.http_status(false), 400);
        error.code = PluginErrorCode::ResourceLimit;
        assert_eq!(error.http_status(false), 502);
        assert_eq!(error.http_status(true), 429);
        error.code = PluginErrorCode::Timeout;
        assert_eq!(error.http_status(false), 504);
        error.details = Some(json!({"oversized": "x".repeat(5000)}));
        assert!(error.validate().is_err());
        error.details = None;
        error.message = "x".repeat(513);
        assert!(error.validate().is_err());
        assert!(
            serde_json::from_value::<PluginError>(json!({"code": "arbitrary", "message": "oops"}))
                .is_err()
        );
    }
}
