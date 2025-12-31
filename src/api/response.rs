//! API response types

use serde::{Deserialize, Serialize};

/// Standard Cloudflare API response
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(default)]
    pub errors: Vec<ApiError>,
    #[serde(default)]
    pub messages: Vec<ApiMessage>,
    pub result: Option<T>,
    pub result_info: Option<ResultInfo>,
}

/// API error
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub code: i32,
    pub message: String,
}

/// API message
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiMessage {
    pub code: Option<i32>,
    pub message: String,
}

/// Pagination info
#[derive(Debug, Serialize, Deserialize)]
pub struct ResultInfo {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub total_pages: Option<u32>,
    pub count: Option<u32>,
    pub total_count: Option<u32>,
}
