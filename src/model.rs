use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostContext {
    pub session: String,
    pub document: String,
    #[serde(default)]
    pub open: bool,
    pub title: Option<String>,
    #[serde(rename = "fileUrl")]
    pub file_url: Option<String>,
    pub protocol: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Observed,
    AppliedVerified,
    AppliedUnverified,
    NotApplied,
    #[serde(rename = "outcome_unknown")]
    Unknown,
    PartiallyApplied,
}

#[derive(Debug, Serialize)]
pub struct Receipt {
    pub request_id: String,
    pub outcome: Outcome,
    pub session: Option<String>,
    pub document: Option<String>,
    pub result: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<OperationError>,
    pub elapsed_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[error("{code}: {message}")]
pub struct OperationError {
    pub code: String,
    pub message: String,
}

impl OperationError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
