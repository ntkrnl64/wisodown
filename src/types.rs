use serde::Deserialize;

// ── Internal API response types ────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct ApiErrors {
    #[serde(default)]
    pub errors: Vec<ApiError>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct ApiError {
    pub key: String,
    pub value: String,
    #[serde(rename = "Type")]
    pub error_type: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct SkuResponse {
    pub skus: Vec<Sku>,
}

// ── Public types ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Sku {
    pub id: String,
    pub language: String,
    pub localized_language: String,
    pub friendly_file_names: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DownloadResponse {
    #[serde(default)]
    pub product_download_options: Vec<DownloadOption>,
    pub download_expiration_datetime: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DownloadOption {
    pub name: String,
    pub uri: String,
    #[allow(dead_code)]
    pub language: String,
    #[allow(dead_code)]
    pub download_type: i32,
}

// ── Dynamic configuration extracted from the page / JS bundle ──────────────

pub(crate) struct PageConfig {
    pub api_base: String,
    pub profile: String,
    pub instance_id: String,
    pub org_id: String,
}

impl Default for PageConfig {
    fn default() -> Self {
        Self {
            api_base: "https://www.microsoft.com/software-download-connector/api".into(),
            profile: "606624d44113".into(),
            instance_id: "560dc9f3-1aa5-4a2f-b63c-9e18f8d0e175".into(),
            org_id: "y6jn8c31".into(),
        }
    }
}

// ── Cancellation sentinel ──────────────────────────────────────────────────

#[derive(Debug)]
pub struct Cancelled;
impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cancelled by user")
    }
}
impl std::error::Error for Cancelled {}
