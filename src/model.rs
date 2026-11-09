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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn receipt_json_transport_preserves_promoted_normalized_and_exact_f64_bits() {
        let normalized = f64::from(f32::from_bits(0x307D_AB0C));
        for bits in [
            normalized.to_bits(), // The Unit13 normalized shadow lost two f64 ULPs.
            0x3FB9_9999_9999_999A,
            0x3FB9_9999_9999_999B,
            0x3FEF_FFFF_FFFF_FFFF,
            0x3FF0_0000_0000_0001,
            0x8000_0000_0000_0000,
            0x0000_0000_0000_0001,
            0x000F_FFFF_FFFF_FFFF,
            0x0010_0000_0000_0000,
            0x7FEF_FFFF_FFFF_FFFF,
            0xFFEF_FFFF_FFFF_FFFF,
        ] {
            let receipt = Receipt {
                request_id: "numeric-transport".into(),
                outcome: Outcome::Observed,
                session: None,
                document: None,
                result: json!({"value": f64::from_bits(bits), "normalized": normalized,
                    "native_normalized": {"kind": "float32", "bits_hex": "307DAB0C"}}),
                error: None,
                elapsed_ms: 0,
            };
            let mut wire = serde_json::to_string(&receipt).unwrap();
            for _ in 0..2 {
                let parsed: Value = serde_json::from_str(&wire).unwrap();
                assert_eq!(parsed["result"]["value"].as_f64().unwrap().to_bits(), bits);
                assert_eq!(
                    parsed["result"]["normalized"].as_f64().unwrap().to_bits(),
                    normalized.to_bits()
                );
                assert_eq!(
                    parsed["result"]["native_normalized"]["bits_hex"],
                    "307DAB0C"
                );
                wire = serde_json::to_string(&parsed).unwrap();
            }
        }
    }
}
