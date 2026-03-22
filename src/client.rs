use anyhow::{bail, Context, Result};
use reqwest::header;
use std::collections::HashMap;

use crate::parse::{
    extract_from_sds_js, extract_hidden_input, extract_sds_js_url, parse_page_hashes,
};
use crate::types::{ApiErrors, DownloadResponse, PageConfig, Sku, SkuResponse};

pub struct MsDownloadClient {
    pub(crate) http: reqwest::Client,
    pub(crate) session_id: String,
    pub(crate) page_url: String,
    cfg: PageConfig,
    pub(crate) browser_cookie: Option<String>,
    pub(crate) debug: bool,
}

impl MsDownloadClient {
    pub async fn init(
        page_url: String,
        browser_cookie: Option<String>,
        debug: bool,
    ) -> Result<Self> {
        let session_id = uuid::Uuid::new_v4().to_string();

        let http = reqwest::Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
                 AppleWebKit/537.36 (KHTML, like Gecko) \
                 Chrome/134.0.0.0 Safari/537.36",
            )
            .build()?;

        if debug {
            eprintln!("[debug] session_id: {session_id}");
            eprintln!("[debug] Loading page: {page_url}");
        }

        // ── 1. Load the download page ──────────────────────────────────────
        let page_html = http
            .get(&page_url)
            .header(header::ACCEPT, "text/html,*/*;q=0.8")
            .header(header::ACCEPT_LANGUAGE, "en-US,en;q=0.5")
            .send()
            .await
            .context("Failed to load the Microsoft download page")?
            .text()
            .await
            .context("Failed to read page body")?;

        if debug {
            eprintln!("[debug] Page: {} bytes", page_html.len());
        }

        // ── 2. Extract endpoint-svc (API base) from the page ───────────────
        let mut cfg = PageConfig::default();
        if let Some(ep) = extract_hidden_input(&page_html, "endpoint-svc") {
            if debug {
                eprintln!("[debug] endpoint-svc: {ep}");
            }
            cfg.api_base = ep.trim_end_matches('/').to_string();
        }

        // ── 3. Fetch the SDS JS bundle and extract profile / IDs ───────────
        if let Some(js_url) = extract_sds_js_url(&page_html) {
            if debug {
                eprintln!("[debug] SDS JS: {js_url}");
            }
            match http.get(&js_url).send().await {
                Ok(resp) => {
                    if let Ok(js) = resp.text().await {
                        let api_base = cfg.api_base.clone();
                        cfg = extract_from_sds_js(&js);
                        cfg.api_base = api_base;
                        if debug {
                            eprintln!("[debug] profile: {}", cfg.profile);
                            eprintln!("[debug] instanceId: {}", cfg.instance_id);
                            eprintln!("[debug] orgId: {}", cfg.org_id);
                        }
                    }
                }
                Err(e) if debug => eprintln!("[debug] Failed to fetch SDS JS: {e}"),
                _ => {}
            }
        }

        let client = Self {
            http,
            session_id,
            page_url,
            cfg,
            browser_cookie,
            debug,
        };

        // ── 4. Run both fingerprint flows ──────────────────────────────────
        client.get_vlsc_fingerprint().await;
        client.get_ov_df_fingerprint().await?;

        Ok(client)
    }

    // ── VLSC / ThreatMetrix fingerprint ────────────────────────────────────

    async fn get_vlsc_fingerprint(&self) {
        let tags_js = format!(
            "https://vlscppe.microsoft.com/fp/tags.js?org_id={}&session_id={}",
            self.cfg.org_id, self.session_id
        );
        let tags_iframe = format!(
            "https://vlscppe.microsoft.com/tags?org_id={}&session_id={}",
            self.cfg.org_id, self.session_id
        );

        if self.debug {
            eprintln!("[debug] VLSC tags.js: {tags_js}");
        }

        if let Ok(r) = self
            .http
            .get(&tags_js)
            .header(header::REFERER, self.page_url.as_str())
            .send()
            .await
        {
            r.bytes().await.ok();
        }

        if self.debug {
            eprintln!("[debug] VLSC tags iframe: {tags_iframe}");
        }

        if let Ok(r) = self
            .http
            .get(&tags_iframe)
            .header(header::REFERER, self.page_url.as_str())
            .send()
            .await
        {
            r.bytes().await.ok();
        }
    }

    // ── ov-df fingerprint (mdt.js → doFpt) ────────────────────────────────

    async fn get_ov_df_fingerprint(&self) -> Result<()> {
        let mdt_url = format!(
            "https://ov-df.microsoft.com/mdt.js\
             ?instanceId={}\
             &pageId=si\
             &session_id={}",
            self.cfg.instance_id, self.session_id
        );

        if self.debug {
            eprintln!("[debug] mdt.js: {mdt_url}");
        }

        let mdt_body = self
            .http
            .get(&mdt_url)
            .header(header::REFERER, self.page_url.as_str())
            .header(header::ACCEPT, "*/*")
            .send()
            .await
            .context("Failed to contact fingerprint service (mdt.js)")?
            .text()
            .await
            .context("Failed to read mdt.js body")?;

        if self.debug {
            eprintln!(
                "[debug] mdt.js ({} bytes): {}",
                mdt_body.len(),
                &mdt_body[..mdt_body.len().min(500)]
            );
        }

        let ticks: String = mdt_body
            .find("&w=")
            .and_then(|i| {
                let rest = &mdt_body[i + 3..];
                rest.find(|c: char| !c.is_ascii_hexdigit())
                    .map(|j| rest[..j].to_string())
            })
            .or_else(|| {
                mdt_body.find("ticks:'").and_then(|i| {
                    let rest = &mdt_body[i + 7..];
                    rest.find('\'').map(|j| rest[..j].to_string())
                })
            })
            .unwrap_or_else(|| "8DE86C5F9B41AD4".to_string());

        if self.debug {
            eprintln!("[debug] ticks: {ticks}");
        }

        #[cfg(not(target_arch = "wasm32"))]
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        #[cfg(target_arch = "wasm32")]
        let now_ms = js_sys::Date::now() as u128;

        let fpt_url = format!(
            "https://ov-df.microsoft.com/\
             ?session_id={}\
             &CustomerId={}\
             &PageId=si\
             &w={ticks}\
             &mdt={now_ms}\
             &rticks={now_ms}",
            self.session_id, self.cfg.instance_id
        );

        if self.debug {
            eprintln!("[debug] fpt iframe: {fpt_url}");
        }

        let resp = self
            .http
            .get(&fpt_url)
            .header(header::REFERER, self.page_url.as_str())
            .send()
            .await
            .context("Failed to contact fingerprint service")?;

        if self.debug {
            eprintln!("[debug] fpt status: {}", resp.status());
        }

        resp.bytes().await.ok();
        Ok(())
    }

    // ── Page hash scraping ─────────────────────────────────────────────────

    pub async fn fetch_page_hashes(&self) -> HashMap<String, String> {
        if self.debug {
            eprintln!("[debug] Fetching page hashes from: {}", self.page_url);
        }
        let html = match self
            .http
            .get(&self.page_url)
            .header(header::ACCEPT, "text/html,*/*;q=0.8")
            .header(header::ACCEPT_LANGUAGE, "en-US,en;q=0.5")
            .send()
            .await
        {
            Ok(r) => match r.text().await {
                Ok(t) => t,
                Err(_) => return HashMap::new(),
            },
            Err(_) => return HashMap::new(),
        };

        let map = parse_page_hashes(&html);
        if self.debug {
            eprintln!("[debug] Found {} hash entries in page", map.len());
        }
        map
    }

    // ── API endpoints ──────────────────────────────────────────────────────

    pub async fn get_skus(&self, product_edition_id: &str) -> Result<Vec<Sku>> {
        let url = format!(
            "{}/getskuinformationbyproductedition\
             ?profile={}\
             &ProductEditionId={product_edition_id}\
             &SKU=undefined\
             &friendlyFileName=undefined\
             &Locale=en-US\
             &sessionID={}",
            self.cfg.api_base, self.cfg.profile, self.session_id
        );

        if self.debug {
            eprintln!("[debug] GET {url}");
        }

        let mut req = self
            .http
            .get(&url)
            .header(header::ACCEPT_LANGUAGE, "en-US,en;q=0.9")
            .header(header::REFERER, self.page_url.as_str())
            .header("X-Requested-With", "XMLHttpRequest")
            .header("Sec-Fetch-Site", "same-origin")
            .header("Sec-Fetch-Mode", "cors")
            .header("Sec-Fetch-Dest", "empty");
        if let Some(c) = &self.browser_cookie {
            req = req.header(header::COOKIE, c);
        }
        let body = req
            .send()
            .await
            .context("Failed to contact Microsoft API (SKU endpoint)")?
            .text()
            .await
            .context("Failed to read SKU response body")?;

        if self.debug {
            eprintln!(
                "[debug] SKU response ({} bytes):\n{}",
                body.len(),
                &body[..body.len().min(1000)]
            );
        }

        Self::check_api_errors(&body)?;

        let resp: SkuResponse = serde_json::from_str(&body).with_context(|| {
            format!(
                "Failed to parse SKU JSON.\nBody ({} bytes): {}",
                body.len(),
                &body[..body.len().min(500)]
            )
        })?;

        if resp.skus.is_empty() {
            bail!("No SKUs returned – the product edition ID may be invalid or expired");
        }
        Ok(resp.skus)
    }

    pub async fn get_download_links(&self, sku_id: &str) -> Result<DownloadResponse> {
        let url = format!(
            "{}/GetProductDownloadLinksBySku\
             ?profile={}\
             &ProductEditionId=undefined\
             &SKU={sku_id}\
             &friendlyFileName=undefined\
             &Locale=en-US\
             &sessionID={}",
            self.cfg.api_base, self.cfg.profile, self.session_id
        );

        if self.debug {
            eprintln!("[debug] GET {url}");
        }

        let mut req = self
            .http
            .get(&url)
            .header(
                header::ACCEPT,
                "application/json, text/javascript, */*; q=0.01",
            )
            .header(header::ACCEPT_LANGUAGE, "en-US,en;q=0.9")
            .header(header::REFERER, self.page_url.as_str())
            .header("X-Requested-With", "XMLHttpRequest")
            .header("Sec-Fetch-Site", "same-origin")
            .header("Sec-Fetch-Mode", "cors")
            .header("Sec-Fetch-Dest", "empty");
        if let Some(c) = &self.browser_cookie {
            req = req.header(header::COOKIE, c);
        }
        let body = req
            .send()
            .await
            .context("Failed to contact Microsoft download API")?
            .text()
            .await
            .context("Failed to read download response body")?;

        if self.debug {
            eprintln!(
                "[debug] Download response ({} bytes):\n{}",
                body.len(),
                &body[..body.len().min(1000)]
            );
        }

        Self::check_api_errors(&body)?;

        let resp: DownloadResponse = serde_json::from_str(&body).with_context(|| {
            format!(
                "Failed to parse download JSON.\nBody ({} bytes): {}",
                body.len(),
                &body[..body.len().min(500)]
            )
        })?;

        if resp.product_download_options.is_empty() {
            bail!("No download links returned – the SKU may be invalid");
        }
        Ok(resp)
    }

    fn check_api_errors(body: &str) -> Result<()> {
        if let Ok(err_resp) = serde_json::from_str::<ApiErrors>(body) {
            if !err_resp.errors.is_empty() {
                let msgs: Vec<String> = err_resp
                    .errors
                    .iter()
                    .map(|e| format!("[{}] {} (type {})", e.key, e.value, e.error_type))
                    .collect();
                bail!("Microsoft API returned error(s):\n  {}", msgs.join("\n  "));
            }
        }
        Ok(())
    }
}
