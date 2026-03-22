use anyhow::{bail, Context, Result};
use clap::Parser;
use console::Style;
use dialoguer::{theme::ColorfulTheme, Select};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::header;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// ── Known edition constants ────────────────────────────────────────────────
const PAGE_URL_WIN10: &str = "https://www.microsoft.com/en-us/software-download/windows10ISO";
const PAGE_URL_X64: &str = "https://www.microsoft.com/en-us/software-download/windows11";
const PAGE_URL_ARM64: &str = "https://www.microsoft.com/en-us/software-download/windows11arm64";

const EDITION_WIN10: &str = "2618";
const EDITION_X64: &str = "3321";
const EDITION_ARM64: &str = "3324";
const EDITION_CN_HOME: &str = "3322";
const EDITION_CN_PRO: &str = "3323";

// ── CLI definition ─────────────────────────────────────────────────────────
/// Windows ISO Downloader — fetch Windows 10 and Windows 11 ISOs directly
/// from Microsoft's servers.
///
/// Run without arguments for an interactive wizard, or use flags to skip
/// all prompts.  --edition and --edition-id are mutually exclusive.
#[derive(Parser, Debug)]
#[command(name = "wisodown", version, about, long_about = None)]
struct Cli {
    /// Named edition alias: "x64", "arm64" (Windows 11), "win10",
    /// "win11-cn-home", "win11-cn-pro".
    /// Mutually exclusive with --edition-id.
    #[arg(short, long, conflicts_with = "edition_id")]
    edition: Option<String>,

    /// Raw numeric ProductEditionId from Microsoft's API.
    /// Use --page-url alongside this when the target page differs from the
    /// Windows 11 x64 default.
    #[arg(long, conflicts_with = "edition")]
    edition_id: Option<String>,

    /// Download page URL used for cookie acquisition when --edition-id is set.
    /// Defaults to the Windows 11 x64 page.
    #[arg(long, requires = "edition_id")]
    page_url: Option<String>,

    /// Language name exactly as Microsoft lists it, e.g. "English",
    /// "French", "Japanese".  Case-insensitive.
    #[arg(short, long)]
    language: Option<String>,

    /// Only print the download URL – don't download the file.
    #[arg(long, default_value_t = false)]
    url_only: bool,

    /// Output directory (defaults to current directory).
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// List available languages for the chosen edition then exit.
    #[arg(long, default_value_t = false)]
    list_languages: bool,

    /// Browser cookies to send with API requests.
    #[arg(long)]
    cookie: Option<String>,

    /// Assert an expected SHA-256 hex digest (overrides the page hash).
    #[arg(long)]
    verify: Option<String>,

    /// Skip hash verification entirely (don't fetch hashes, don't compute
    /// SHA-256 during download).
    #[arg(long, default_value_t = false)]
    no_verify: bool,

    /// Number of parallel download threads (default: 8).
    /// Uses HTTP range requests; falls back to 1 if the server doesn't
    /// support them.
    #[arg(short = 't', long, default_value_t = 8)]
    threads: usize,

    /// Print raw API responses for debugging.
    #[arg(long, default_value_t = false)]
    debug: bool,
}

// ── Cancellation sentinel ──────────────────────────────────────────────────

#[derive(Debug)]
struct Cancelled;
impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cancelled by user")
    }
}
impl std::error::Error for Cancelled {}

// ── API response types ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiErrors {
    #[serde(default)]
    errors: Vec<ApiError>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiError {
    key: String,
    value: String,
    #[serde(rename = "Type")]
    error_type: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SkuResponse {
    skus: Vec<Sku>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
struct Sku {
    id: String,
    language: String,
    localized_language: String,
    friendly_file_names: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct DownloadResponse {
    #[serde(default)]
    product_download_options: Vec<DownloadOption>,
    download_expiration_datetime: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct DownloadOption {
    name: String,
    uri: String,
    #[allow(dead_code)]
    language: String,
    #[allow(dead_code)]
    download_type: i32,
}

// ── Dynamic configuration extracted from the page / JS bundle ──────────────

struct PageConfig {
    api_base: String,
    profile: String,
    instance_id: String,
    org_id: String,
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

/// Extract the value of a hidden `<input>` by its `id` attribute.
fn extract_hidden_input(html: &str, id: &str) -> Option<String> {
    let pattern = format!("id=\"{id}\"");
    let idx = html.find(&pattern)?;
    let region = &html[idx..html.len().min(idx + 300)];
    let val_start = region.find("value=\"")? + 7;
    let val_end = val_start + region[val_start..].find('"')?;
    let val = region[val_start..val_end].trim();
    if val.is_empty() {
        None
    } else {
        Some(val.to_string())
    }
}

/// Find the SDS JS bundle URL on the page.
///
/// Looks for `<script src="…sdsbase…site.ACSHASH….min.js">` specifically,
/// skipping the CSS variant.
fn extract_sds_js_url(html: &str) -> Option<String> {
    let marker = "sdsbase/v1/sdsbase/clientlibs/site.";
    let mut search = 0;
    while let Some(rel) = html[search..].find(marker) {
        let marker_pos = search + rel;
        // Verify this is the .js file, not .css
        let tail = &html[marker_pos..html.len().min(marker_pos + 120)];
        if tail.contains(".min.js") {
            // Walk back at most 300 chars to find src=" in the same tag
            let win_start = marker_pos.saturating_sub(300);
            let window = &html[win_start..marker_pos];
            if let Some(src_rel) = window.rfind("src=\"") {
                let src_start = win_start + src_rel + 5;
                if let Some(src_end) = html[src_start..].find('"') {
                    let path = html[src_start..src_start + src_end].trim();
                    return if path.starts_with("http") {
                        Some(path.to_string())
                    } else {
                        Some(format!("https://www.microsoft.com{path}"))
                    };
                }
            }
        }
        search = marker_pos + marker.len();
    }
    None
}

/// Extract profile, instanceId, and orgId from the SDS JS bundle text.
fn extract_from_sds_js(js: &str) -> PageConfig {
    let mut cfg = PageConfig::default();

    // profile: `profile\x3d606624d44113\x26`
    if let Some(rest) = js
        .find("profile\\x3d")
        .map(|i| &js[i + 11..])
        .or_else(|| js.find("profile=").map(|i| &js[i + 8..]))
    {
        if let Some(end) = rest.find(|c: char| !c.is_ascii_alphanumeric()) {
            let v = &rest[..end];
            if !v.is_empty() {
                cfg.profile = v.to_string();
            }
        }
    }

    // instanceId: UUID after `instanceId\x3d`
    if let Some(rest) = js
        .find("instanceId\\x3d")
        .map(|i| &js[i + 14..])
        .or_else(|| js.find("instanceId=").map(|i| &js[i + 11..]))
    {
        if let Some(end) = rest.find(|c: char| !c.is_ascii_alphanumeric() && c != '-') {
            let v = &rest[..end];
            if !v.is_empty() {
                cfg.instance_id = v.to_string();
            }
        }
    }

    // orgId: first non-empty `orgId:"VALUE"`
    let mut search = js;
    while let Some(pos) = search.find("orgId:\"") {
        let rest = &search[pos + 7..];
        if let Some(end) = rest.find('"') {
            let v = &rest[..end];
            if !v.is_empty() {
                cfg.org_id = v.to_string();
                break;
            }
        }
        search = &search[pos + 7..];
    }

    cfg
}

// ── API client ─────────────────────────────────────────────────────────────

struct MsDownloadClient {
    http: reqwest::Client,
    session_id: String,
    page_url: String,
    cfg: PageConfig,
    browser_cookie: Option<String>,
    debug: bool,
}

impl MsDownloadClient {
    async fn init(page_url: String, browser_cookie: Option<String>, debug: bool) -> Result<Self> {
        let session_id = uuid::Uuid::new_v4().to_string();

        let http = reqwest::Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
                 AppleWebKit/537.36 (KHTML, like Gecko) \
                 Chrome/134.0.0.0 Safari/537.36",
            )
            .cookie_store(true)
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
                        cfg.api_base = api_base; // keep page-derived api_base
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

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();

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

    async fn fetch_page_hashes(&self) -> HashMap<String, String> {
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

    async fn get_skus(&self, product_edition_id: &str) -> Result<Vec<Sku>> {
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

    async fn get_download_links(&self, sku_id: &str) -> Result<DownloadResponse> {
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

    // ── Download ───────────────────────────────────────────────────────────

    async fn download_file(
        &self,
        url: &str,
        temp_path: &std::path::Path,
        threads: usize,
        compute_hash: bool,
    ) -> Result<Option<String>> {
        // HEAD to discover Content-Length and Range support
        let head = self
            .http
            .head(url)
            .send()
            .await
            .context("HEAD request failed")?;
        let total = head.content_length().unwrap_or(0);
        let accepts_ranges = head
            .headers()
            .get(header::ACCEPT_RANGES)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.contains("bytes"))
            .unwrap_or(false);

        let use_multi = threads > 1 && accepts_ranges && total > 1024 * 1024;

        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta}) {bytes_per_sec}",
            )?
            .progress_chars("█▓░"),
        );

        let ctrl_c = tokio::signal::ctrl_c();
        tokio::pin!(ctrl_c);

        let inline_hash: Option<String>;

        if use_multi {
            // ── Multi-threaded download ────────────────────────────────
            {
                let f = std::fs::File::create(temp_path)
                    .with_context(|| format!("Cannot create {}", temp_path.display()))?;
                f.set_len(total)?;
            }

            let effective_threads = threads.min(total.div_ceil(1024 * 1024) as usize).max(1);
            let chunk_size = total.div_ceil(effective_threads as u64);

            let tasks: Vec<_> = (0..effective_threads)
                .map(|i| {
                    let start = i as u64 * chunk_size;
                    let end = std::cmp::min(start + chunk_size - 1, total - 1);
                    let http = self.http.clone();
                    let url = url.to_string();
                    let path = temp_path.to_path_buf();
                    let pb = pb.clone();

                    tokio::spawn(async move {
                        let resp = http
                            .get(&url)
                            .header(header::RANGE, format!("bytes={start}-{end}"))
                            .send()
                            .await
                            .with_context(|| format!("Chunk {i} request failed"))?;

                        let file = {
                            let mut f = std::fs::OpenOptions::new()
                                .write(true)
                                .open(&path)
                                .with_context(|| {
                                    format!("Cannot open {} for chunk {i}", path.display())
                                })?;
                            use std::io::Seek;
                            f.seek(std::io::SeekFrom::Start(start))?;
                            tokio::fs::File::from_std(f)
                        };
                        let mut file = tokio::io::BufWriter::new(file);
                        let mut stream = resp.bytes_stream();
                        while let Some(chunk) = stream.next().await {
                            let chunk = chunk.context("Error reading chunk stream")?;
                            file.write_all(&chunk).await?;
                            pb.inc(chunk.len() as u64);
                        }
                        file.flush().await?;
                        Ok::<_, anyhow::Error>(())
                    })
                })
                .collect();

            let download = async {
                for task in tasks {
                    task.await
                        .context("Download task panicked")?
                        .context("Download chunk failed")?;
                }
                Ok::<_, anyhow::Error>(())
            };
            tokio::pin!(download);

            tokio::select! {
                biased;
                _ = &mut ctrl_c => {
                    pb.abandon_with_message("cancelled");
                    tokio::fs::remove_file(temp_path).await.ok();
                    return Err(anyhow::Error::new(Cancelled));
                }
                result = &mut download => { result?; }
            }

            inline_hash = None;
        } else {
            // ── Single-threaded streaming download ─────────────────────
            let resp = self
                .http
                .get(url)
                .send()
                .await
                .context("Failed to start download")?;

            let mut file = tokio::fs::File::create(temp_path)
                .await
                .with_context(|| format!("Cannot create {}", temp_path.display()))?;

            let mut hasher = if compute_hash {
                Some(Sha256::new())
            } else {
                None
            };
            let mut stream = resp.bytes_stream();

            loop {
                tokio::select! {
                    biased;
                    _ = &mut ctrl_c => {
                        pb.abandon_with_message("cancelled");
                        drop(file);
                        tokio::fs::remove_file(temp_path).await.ok();
                        return Err(anyhow::Error::new(Cancelled));
                    }
                    chunk = stream.next() => {
                        match chunk {
                            Some(Ok(chunk)) => {
                                if let Some(h) = &mut hasher { h.update(&chunk); }
                                file.write_all(&chunk).await?;
                                pb.inc(chunk.len() as u64);
                            }
                            Some(Err(e)) => bail!("Error reading download stream: {e}"),
                            None => break,
                        }
                    }
                }
            }

            inline_hash = hasher.map(|h| hex::encode(h.finalize()));
        }

        pb.finish_with_message("done");

        // Compute hash if multi-threaded (second pass)
        let hash = if compute_hash {
            match inline_hash {
                Some(h) => Some(h),
                None => Some(compute_file_hash(temp_path).await?),
            }
        } else {
            None
        };

        Ok(hash)
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

async fn compute_file_hash(path: &std::path::Path) -> Result<String> {
    let mut file = tokio::fs::File::open(path)
        .await
        .with_context(|| format!("Cannot open {} for hashing", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 8 * 1024 * 1024];
    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn parse_page_hashes(html: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut pos = 0;
    while pos < html.len() {
        let Some(rel) = html[pos..].find("<td>") else {
            break;
        };
        let lang_start = pos + rel + 4;
        let Some(rel2) = html[lang_start..].find("</td>") else {
            break;
        };
        let lang_end = lang_start + rel2;
        let lang = html[lang_start..lang_end].trim();
        let after_lang = lang_end + 5;

        if after_lang < html.len() && html[after_lang..].starts_with("<td>") {
            let hash_start = after_lang + 4;
            if let Some(rel3) = html[hash_start..].find("</td>") {
                let hash_end = hash_start + rel3;
                let hash = html[hash_start..hash_end].trim();
                if hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
                    map.insert(lang.to_ascii_lowercase(), hash.to_ascii_lowercase());
                }
                pos = hash_end + 5;
                continue;
            }
        }
        pos = lang_end + 5;
    }
    map
}

fn lookup_page_hash(
    hashes: &HashMap<String, String>,
    language: &str,
    option_name: &str,
) -> Option<String> {
    let lang_lower = language.to_ascii_lowercase();
    let name_lower = option_name.to_ascii_lowercase();
    let arch = if name_lower.contains("32-bit") || name_lower.contains("32 bit") {
        "32-bit"
    } else {
        "64-bit"
    };
    let exact = format!("{lang_lower} {arch}");
    if let Some(h) = hashes.get(&exact) {
        return Some(h.clone());
    }
    hashes
        .iter()
        .find(|(k, _)| k.starts_with(&lang_lower) && k.ends_with(arch))
        .map(|(_, v)| v.clone())
}

fn resolve_edition(input: &str) -> Result<(&'static str, &'static str)> {
    match input.to_ascii_lowercase().as_str() {
        "x64" | "x86_64" | "amd64" | "64" | "win11" | "win11-x64" => {
            Ok((EDITION_X64, PAGE_URL_X64))
        }
        "arm64" | "arm" | "aarch64" | "win11-arm64" => Ok((EDITION_ARM64, PAGE_URL_ARM64)),
        "win10" | "windows10" | "10" => Ok((EDITION_WIN10, PAGE_URL_WIN10)),
        "win11-cn-home" | "cn-home" => Ok((EDITION_CN_HOME, PAGE_URL_X64)),
        "win11-cn-pro" | "cn-pro" => Ok((EDITION_CN_PRO, PAGE_URL_X64)),
        _ => bail!(
            "Unknown edition '{}'. Use 'x64', 'arm64', 'win10', \
             'win11-cn-home', 'win11-cn-pro', or --edition-id <N>.",
            input
        ),
    }
}

fn find_sku_by_language<'a>(skus: &'a [Sku], language: &str) -> Result<&'a Sku> {
    let lang_lower = language.to_ascii_lowercase();
    if let Some(s) = skus
        .iter()
        .find(|s| s.language.to_ascii_lowercase() == lang_lower)
    {
        return Ok(s);
    }
    if let Some(s) = skus
        .iter()
        .find(|s| s.localized_language.to_ascii_lowercase() == lang_lower)
    {
        return Ok(s);
    }
    if let Some(s) = skus.iter().find(|s| {
        s.language.to_ascii_lowercase().starts_with(&lang_lower)
            || s.localized_language
                .to_ascii_lowercase()
                .starts_with(&lang_lower)
    }) {
        return Ok(s);
    }
    bail!(
        "Language '{}' not found. Run with --list-languages to see options.",
        language
    );
}

fn filename_from_url(url: &str) -> String {
    url.split('?')
        .next()
        .and_then(|p| p.rsplit('/').next())
        .unwrap_or("windows.iso")
        .to_string()
}

// ── Entrypoint ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let bold = Style::new().bold();
    let cyan = Style::new().cyan();
    let green = Style::new().green().bold();
    let red = Style::new().red().bold();
    let dim = Style::new().dim();

    // ── 1. Resolve edition + page URL ──────────────────────────────────────
    let (edition_id, page_url): (String, String) = if let Some(id) = &cli.edition_id {
        let page = cli
            .page_url
            .clone()
            .unwrap_or_else(|| PAGE_URL_X64.to_string());
        (id.clone(), page)
    } else {
        let (eid, purl) = match &cli.edition {
            Some(e) => resolve_edition(e)?,
            None => {
                let labels = [
                    "Windows 11 (x64)",
                    "Windows 11 (arm64)",
                    "Windows 11 Home (China)",
                    "Windows 11 Pro (China)",
                    "Windows 10",
                ];
                let keys = ["x64", "arm64", "win11-cn-home", "win11-cn-pro", "win10"];
                let selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select Windows version")
                    .items(&labels)
                    .default(0)
                    .interact()?;
                resolve_edition(keys[selection])?
            }
        };
        (eid.to_string(), purl.to_string())
    };

    let (os_label, arch_label) = match edition_id.as_str() {
        EDITION_X64 => ("Windows 11", "x64"),
        EDITION_ARM64 => ("Windows 11", "arm64"),
        EDITION_WIN10 => ("Windows 10", "multi-edition"),
        EDITION_CN_HOME => ("Windows 11 Home", "China"),
        EDITION_CN_PRO => ("Windows 11 Pro", "China"),
        _ => ("Windows", edition_id.as_str()),
    };
    eprintln!(
        "{} Initializing session for {os_label} ({arch_label})…",
        cyan.apply_to("➜")
    );

    // ── 2. Initialize client ───────────────────────────────────────────────
    let client = MsDownloadClient::init(page_url, cli.cookie, cli.debug).await?;

    eprintln!("{} Fetching available languages…", cyan.apply_to("➜"));

    // ── 3. Fetch SKUs ──────────────────────────────────────────────────────
    let skus = client.get_skus(&edition_id).await?;

    if cli.list_languages {
        println!("{}", bold.apply_to("Available languages:"));
        for sku in &skus {
            println!(
                "  {:<30} (id: {}, file: {})",
                sku.localized_language,
                sku.id,
                sku.friendly_file_names.first().unwrap_or(&String::new())
            );
        }
        return Ok(());
    }

    // ── 4. Resolve language ────────────────────────────────────────────────
    let sku = match &cli.language {
        Some(lang) => find_sku_by_language(&skus, lang)?.clone(),
        None => {
            let labels: Vec<String> = skus
                .iter()
                .map(|s| format!("{} ({})", s.localized_language, s.language))
                .collect();
            let default_idx = skus
                .iter()
                .position(|s| s.language == "English")
                .unwrap_or(0);
            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select language")
                .items(&labels)
                .default(default_idx)
                .interact()?;
            skus[selection].clone()
        }
    };

    eprintln!(
        "{} Selected: {} (SKU {})",
        cyan.apply_to("➜"),
        bold.apply_to(&sku.localized_language),
        sku.id
    );

    // ── 5. Get download link ───────────────────────────────────────────────
    eprintln!("{} Requesting download link…", cyan.apply_to("➜"));
    let dl = client.get_download_links(&sku.id).await?;
    let option_idx = if dl.product_download_options.len() > 1 {
        let labels: Vec<String> = dl
            .product_download_options
            .iter()
            .map(|o| o.name.clone())
            .collect();
        Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select download")
            .items(&labels)
            .default(0)
            .interact()?
    } else {
        0
    };
    let option = &dl.product_download_options[option_idx];

    if let Some(exp) = &dl.download_expiration_datetime {
        eprintln!("  {} Link expires: {exp}", dim.apply_to("ℹ"));
    }

    if cli.url_only {
        println!("{}", option.uri);
        return Ok(());
    }

    // ── 6. Fetch page hashes (unless --no-verify) ──────────────────────────
    let compute_hash = !cli.no_verify;
    let expected_hash: Option<String> = if cli.no_verify {
        None
    } else {
        eprintln!(
            "{} Fetching integrity hashes from Microsoft…",
            cyan.apply_to("➜")
        );
        let page_hashes = client.fetch_page_hashes().await;
        let hash = cli
            .verify
            .as_deref()
            .map(|s| s.to_ascii_lowercase())
            .or_else(|| lookup_page_hash(&page_hashes, &sku.language, &option.name));
        if let Some(ref h) = hash {
            eprintln!("  {} Expected SHA-256: {h}", dim.apply_to("ℹ"));
        } else {
            eprintln!("  {} No hash found — will compute only", dim.apply_to("ℹ"));
        }
        hash
    };

    // ── 7. Download to temp file ───────────────────────────────────────────
    let filename = sku
        .friendly_file_names
        .first()
        .cloned()
        .unwrap_or_else(|| filename_from_url(&option.uri));

    let dest = cli
        .output
        .unwrap_or_else(|| PathBuf::from("."))
        .join(&filename);

    let temp = dest.with_extension("iso.part");

    eprintln!(
        "{} Downloading {} → {}",
        green.apply_to("⬇"),
        bold.apply_to(&option.name),
        dest.display()
    );
    if cli.threads > 1 {
        eprintln!(
            "  {} Using up to {} threads",
            dim.apply_to("ℹ"),
            cli.threads
        );
    }

    let sha256 = match client
        .download_file(&option.uri, &temp, cli.threads, compute_hash)
        .await
    {
        Ok(h) => h,
        Err(e) if e.downcast_ref::<Cancelled>().is_some() => {
            eprintln!("\n{} Download cancelled.", red.apply_to("✘"));
            std::process::exit(130);
        }
        Err(e) => {
            tokio::fs::remove_file(&temp).await.ok();
            return Err(e);
        }
    };

    // ── 8. Move temp → final destination ───────────────────────────────────
    tokio::fs::rename(&temp, &dest)
        .await
        .with_context(|| format!("Failed to move {} → {}", temp.display(), dest.display()))?;

    eprintln!(
        "\n{} Saved to {}",
        green.apply_to("✔"),
        bold.apply_to(dest.display())
    );

    // ── 9. Verify hash ─────────────────────────────────────────────────────
    if let Some(sha256) = sha256 {
        eprintln!("  SHA-256: {sha256}");
        match expected_hash {
            Some(expected) if sha256 == expected => {
                eprintln!("{} Hash verified", green.apply_to("✔"));
            }
            Some(expected) => {
                eprintln!(
                    "{} Hash mismatch!\n  expected: {expected}\n  got:      {sha256}",
                    red.apply_to("✘")
                );
                std::process::exit(1);
            }
            None => {}
        }
    }

    Ok(())
}
