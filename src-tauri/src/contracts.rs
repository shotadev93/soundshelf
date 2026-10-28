use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioAsset {
    pub id: String,
    pub name: String,
    pub duration_seconds: Option<f64>,
    pub relative_display_path: String,
    pub extension: String,
    pub availability: Availability,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Availability {
    Available,
    Inaccessible,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    NotAuthorized,
    Unavailable,
    InvalidInput,
    Busy,
    InternalError,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
}

impl AppError {
    pub fn new(code: ErrorCode, message: &str) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn unavailable(message: &str) -> Self {
        Self::new(ErrorCode::Unavailable, message)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryInfo {
    pub id: String,
    pub display_path: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryRequest {
    pub library_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ScanPhase {
    Idle,
    Running,
    Complete,
    Partial,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanWarning {
    pub message: String,
    pub relative_display_path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStatus {
    pub scan_id: Option<String>,
    pub phase: ScanPhase,
    pub visited_entries: u32,
    pub discovered_assets: u32,
    pub warning_count: u32,
    pub warnings: Vec<ScanWarning>,
    pub error: Option<AppError>,
}

impl Default for ScanStatus {
    fn default() -> Self {
        Self {
            scan_id: None,
            phase: ScanPhase::Idle,
            visited_entries: 0,
            discovered_assets: 0,
            warning_count: 0,
            warnings: Vec::new(),
            error: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetList {
    pub scan_id: Option<String>,
    pub assets: Vec<AudioAsset>,
}
