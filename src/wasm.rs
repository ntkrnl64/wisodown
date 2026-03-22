use wasm_bindgen::prelude::*;

use crate::client::MsDownloadClient;
use crate::edition;

/// JavaScript-facing wrapper around [MsDownloadClient].
#[wasm_bindgen]
pub struct WasmClient {
    inner: MsDownloadClient,
}

#[wasm_bindgen]
impl WasmClient {
    /// Create a new client session.
    ///
    /// `page_url` — one of the Microsoft download page URLs (use the
    /// `page_url_*` helpers), or any custom URL.
    ///
    /// `browser_cookie` — optional cookie string to send with API requests.
    #[wasm_bindgen(js_name = "create")]
    pub async fn create(
        page_url: &str,
        browser_cookie: Option<String>,
        debug: Option<bool>,
    ) -> Result<WasmClient, JsError> {
        let inner =
            MsDownloadClient::init(page_url.to_string(), browser_cookie, debug.unwrap_or(false))
                .await
                .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Self { inner })
    }

    /// Fetch available languages (SKUs) for a product edition.
    ///
    /// Returns an array of `{ Id, Language, LocalizedLanguage, FriendlyFileNames }`.
    #[wasm_bindgen(js_name = "getSkus")]
    pub async fn get_skus(&self, product_edition_id: &str) -> Result<JsValue, JsError> {
        let skus = self
            .inner
            .get_skus(product_edition_id)
            .await
            .map_err(|e| JsError::new(&e.to_string()))?;
        serde_wasm_bindgen::to_value(&skus).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Get download links for a given SKU id.
    ///
    /// Returns `{ ProductDownloadOptions, DownloadExpirationDatetime }`.
    #[wasm_bindgen(js_name = "getDownloadLinks")]
    pub async fn get_download_links(&self, sku_id: &str) -> Result<JsValue, JsError> {
        let resp = self
            .inner
            .get_download_links(sku_id)
            .await
            .map_err(|e| JsError::new(&e.to_string()))?;
        serde_wasm_bindgen::to_value(&resp).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Scrape SHA-256 hashes from the download page.
    ///
    /// Returns a `Record<string, string>` mapping lowercase
    /// `"language arch"` keys to hex hash values.
    #[wasm_bindgen(js_name = "fetchPageHashes")]
    pub async fn fetch_page_hashes(&self) -> Result<JsValue, JsError> {
        let hashes = self.inner.fetch_page_hashes().await;
        serde_wasm_bindgen::to_value(&hashes).map_err(|e| JsError::new(&e.to_string()))
    }
}

// ── Free functions ─────────────────────────────────────────────────────────

/// Resolve an edition alias (e.g. `"x64"`, `"arm64"`, `"win10"`) to
/// `{ editionId, pageUrl }`.
#[wasm_bindgen(js_name = "resolveEdition")]
pub fn resolve_edition(input: &str) -> Result<JsValue, JsError> {
    let (edition_id, page_url) =
        edition::resolve_edition(input).map_err(|e| JsError::new(&e.to_string()))?;
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"editionId".into(), &edition_id.into())
        .map_err(|e| JsError::new(&format!("{e:?}")))?;
    js_sys::Reflect::set(&obj, &"pageUrl".into(), &page_url.into())
        .map_err(|e| JsError::new(&format!("{e:?}")))?;
    Ok(obj.into())
}

#[wasm_bindgen(js_name = "pageUrlWin10")]
pub fn page_url_win10() -> String {
    edition::PAGE_URL_WIN10.to_string()
}

#[wasm_bindgen(js_name = "pageUrlX64")]
pub fn page_url_x64() -> String {
    edition::PAGE_URL_X64.to_string()
}

#[wasm_bindgen(js_name = "pageUrlArm64")]
pub fn page_url_arm64() -> String {
    edition::PAGE_URL_ARM64.to_string()
}

#[wasm_bindgen(js_name = "editionWin10")]
pub fn edition_win10() -> String {
    edition::EDITION_WIN10.to_string()
}

#[wasm_bindgen(js_name = "editionX64")]
pub fn edition_x64() -> String {
    edition::EDITION_X64.to_string()
}

#[wasm_bindgen(js_name = "editionArm64")]
pub fn edition_arm64() -> String {
    edition::EDITION_ARM64.to_string()
}
