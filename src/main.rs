use anyhow::{bail, Context, Result};
use clap::Parser;
use console::Style;
use dialoguer::{theme::ColorfulTheme, Select};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::header;
use serde::Deserialize;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

// ── Microsoft API constants ────────────────────────────────────────────────
const API_BASE: &str = "https://www.microsoft.com/software-download-connector/api";
const PROFILE: &str = "606624d44113";

const PAGE_URL_X64: &str = "https://www.microsoft.com/en-us/software-download/windows11";
const PAGE_URL_ARM64: &str = "https://www.microsoft.com/en-us/software-download/windows11arm64";

const EDITION_X64: &str = "3321";
const EDITION_ARM64: &str = "3324";

// ── CLI definition ─────────────────────────────────────────────────────────
/// Download Windows 11 ISOs directly from Microsoft's servers.
///
/// Run without arguments for an interactive wizard, or pass --edition and
/// --language to skip all prompts.
#[derive(Parser, Debug)]
#[command(name = "win11-iso", version, about, long_about = None)]
struct Cli {
    /// Architecture / edition: "x64" or "arm64"
    #[arg(short, long)]
    edition: Option<String>,

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
    ///
    /// Sentinel requires JS-set cookies (MUID, fptctx2, …) that only a real
    /// browser session produces.  Copy them from DevTools → Network → any
    /// request to microsoft.com → Request Headers → Cookie.
    #[arg(long)]
    cookie: Option<String>,

    /// Print raw API responses for debugging.
    #[arg(long, default_value_t = false)]
    debug: bool,
}

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

// ── API client ─────────────────────────────────────────────────────────────

struct MsDownloadClient {
    http: reqwest::Client,
    session_id: String,
    page_url: &'static str,
    browser_cookie: Option<String>,
    debug: bool,
}

impl MsDownloadClient {
    /// Create a new client.
    ///
    /// The session ID is a client-generated UUID v4 (matching what the
    /// browser-side JavaScript does).  We also load the download page so
    /// that reqwest's cookie jar picks up any httpOnly cookies the server
    /// sets — the download-link API (step 2) rejects requests without them.
    async fn init(
        page_url: &'static str,
        browser_cookie: Option<String>,
        debug: bool,
    ) -> Result<Self> {
        let session_id = uuid::Uuid::new_v4().to_string();

        let http = reqwest::Client::builder()
            .user_agent(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
                 AppleWebKit/537.36 (KHTML, like Gecko) \
                 Chrome/134.0.0.0 Safari/537.36",
            )
            .cookie_store(true)
            .build()?;

        if debug {
            eprintln!("[debug] Generated session ID: {session_id}");
            eprintln!("[debug] Loading page to acquire cookies: {page_url}");
        }

        // Load the page to pick up httpOnly cookies.
        let resp = http
            .get(page_url)
            .header(
                header::ACCEPT,
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .header(header::ACCEPT_LANGUAGE, "en-US,en;q=0.5")
            .send()
            .await
            .context("Failed to load the Microsoft download page")?;

        if debug {
            eprintln!("[debug] Page status: {}", resp.status());
            for (k, v) in resp.headers().iter() {
                eprintln!("[debug]   {}: {}", k, v.to_str().unwrap_or("<binary>"));
            }
        }

        let body_len = resp.bytes().await.map(|b| b.len()).unwrap_or(0);
        if debug {
            eprintln!("[debug] Page body: {body_len} bytes (discarded)");
        }

        let client = Self {
            http,
            session_id,
            page_url,
            browser_cookie,
            debug,
        };

        // Obtain fptctx2 + MUID cookies from Microsoft's fingerprint service.
        // These are set by a hidden iframe (window.dfp.doFpt) in the browser;
        // Sentinel rejects API calls that arrive without them.
        client.get_fingerprint_cookie().await?;

        Ok(client)
    }

    /// Simulate the browser's `window.dfp.doFpt()` call.
    ///
    /// The browser loads `ov-df.microsoft.com/mdt.js` to get a JS snippet that
    /// defines `window.dfp`, then injects a hidden iframe whose URL is the
    /// fingerprint endpoint.  That endpoint responds with `Set-Cookie` headers
    /// for `fptctx2` (Sentinel token) and `MUID` on `.microsoft.com`, so they
    /// are automatically sent with every subsequent API request.
    async fn get_fingerprint_cookie(&self) -> Result<()> {
        const INSTANCE_ID: &str = "560dc9f3-1aa5-4a2f-b63c-9e18f8d0e175";

        // ── Step 1: fetch mdt.js to obtain the per-request `ticks` token ──────
        let mdt_url = format!(
            "https://ov-df.microsoft.com/mdt.js\
             ?instanceId={INSTANCE_ID}\
             &pageId=si\
             &session_id={}",
            self.session_id
        );

        if self.debug {
            eprintln!("[debug] Fetching fingerprint JS: {mdt_url}");
        }

        let mdt_body = self
            .http
            .get(&mdt_url)
            .header(header::REFERER, self.page_url)
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

        // Extract the `w` token from the iframe URL embedded in mdt.js:
        //   window.dfp={...,url:"https://ov-df.microsoft.com/?...&w=HEX",...}
        // Fall back to the `ticks:'HEX'` field if the url form isn't found.
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
            eprintln!("[debug] Fingerprint ticks: {ticks}");
        }

        // ── Step 2: call the iframe endpoint that sets fptctx2 + MUID ─────────
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let fpt_url = format!(
            "https://ov-df.microsoft.com/\
             ?session_id={}\
             &CustomerId={INSTANCE_ID}\
             &PageId=si\
             &w={ticks}\
             &mdt={now_ms}\
             &rticks={now_ms}",
            self.session_id
        );

        if self.debug {
            eprintln!("[debug] Fetching fingerprint cookie: {fpt_url}");
        }

        let resp = self
            .http
            .get(&fpt_url)
            .header(header::REFERER, self.page_url)
            .send()
            .await
            .context("Failed to contact fingerprint service")?;

        if self.debug {
            eprintln!("[debug] Fingerprint status: {}", resp.status());
            for (k, v) in resp.headers().iter() {
                eprintln!("[debug]   {}: {}", k, v.to_str().unwrap_or("<binary>"));
            }
        }

        resp.bytes().await.ok();
        Ok(())
    }

    /// Re-visit the download page to refresh the short-lived CAS_PROGRAM cookie
    /// that Sentinel requires on the download-link endpoint.
    async fn refresh_page_cookie(&self) -> Result<()> {
        if self.debug {
            eprintln!("[debug] Refreshing page cookie: {}", self.page_url);
        }
        let resp = self
            .http
            .get(self.page_url)
            .header(
                header::ACCEPT,
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .header(header::ACCEPT_LANGUAGE, "en-US,en;q=0.5")
            .send()
            .await
            .context("Failed to refresh Microsoft download page cookie")?;
        if self.debug {
            eprintln!("[debug] Cookie refresh status: {}", resp.status());
        }
        resp.bytes().await.ok();
        Ok(())
    }

    /// Step 1: Fetch available SKUs (languages) for a product edition.
    async fn get_skus(&self, product_edition_id: &str) -> Result<Vec<Sku>> {
        let url = format!(
            "{API_BASE}/getskuinformationbyproductedition\
             ?profile={PROFILE}\
             &ProductEditionId={product_edition_id}\
             &SKU=undefined\
             &friendlyFileName=undefined\
             &Locale=en-US\
             &sessionID={}",
            self.session_id
        );

        if self.debug {
            eprintln!("[debug] GET {url}");
        }

        let mut req = self
            .http
            .get(&url)
            .header(header::ACCEPT_LANGUAGE, "en-US,en;q=0.9")
            .header(header::REFERER, PAGE_URL_X64)
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

    /// Step 2: Fetch time-limited download link(s) for a SKU.
    async fn get_download_links(&self, sku_id: &str) -> Result<DownloadResponse> {
        // CAS_PROGRAM cookie expires after ~8 seconds; refresh it right before this call.
        self.refresh_page_cookie().await?;

        let url = format!(
            "{API_BASE}/GetProductDownloadLinksBySku\
             ?profile={PROFILE}\
             &ProductEditionId=undefined\
             &SKU={sku_id}\
             &friendlyFileName=undefined\
             &Locale=en-US\
             &sessionID={}",
            self.session_id
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
            .header(header::REFERER, PAGE_URL_X64)
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

    /// Inspect the raw JSON body for Microsoft's `{ "Errors": [...] }` envelope.
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

    /// Stream-download a file with a progress bar.
    async fn download_file(&self, url: &str, dest: &std::path::Path) -> Result<()> {
        let resp = self
            .http
            .get(url)
            .send()
            .await
            .context("Failed to start download")?;

        let total = resp.content_length().unwrap_or(0);
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta}) {bytes_per_sec}",
            )?
            .progress_chars("█▓░"),
        );

        let mut file = tokio::fs::File::create(dest)
            .await
            .with_context(|| format!("Cannot create {}", dest.display()))?;

        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk: bytes::Bytes = chunk.context("Error reading download stream")?;
            file.write_all(&chunk).await?;
            pb.inc(chunk.len() as u64);
        }
        pb.finish_with_message("done");
        Ok(())
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn resolve_edition(input: &str) -> Result<(&'static str, &'static str)> {
    match input.to_ascii_lowercase().as_str() {
        "x64" | "x86_64" | "amd64" | "64" => Ok((EDITION_X64, PAGE_URL_X64)),
        "arm64" | "arm" | "aarch64" => Ok((EDITION_ARM64, PAGE_URL_ARM64)),
        _ => bail!("Unknown edition '{}'. Use 'x64' or 'arm64'.", input),
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
        .unwrap_or("windows11.iso")
        .to_string()
}

// ── Entrypoint ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let bold = Style::new().bold();
    let cyan = Style::new().cyan();
    let green = Style::new().green().bold();

    // ── 1. Resolve edition ─────────────────────────────────────────────────
    let (edition_id, page_url) = match &cli.edition {
        Some(e) => resolve_edition(e)?,
        None => {
            let editions = ["x64", "arm64"];
            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select architecture")
                .items(&editions)
                .default(0)
                .interact()?;
            resolve_edition(editions[selection])?
        }
    };

    let arch_label = if edition_id == EDITION_X64 {
        "x64"
    } else {
        "arm64"
    };
    eprintln!(
        "{} Initializing session for Windows 11 ({arch_label})…",
        cyan.apply_to("➜")
    );

    // ── 2. Initialize client (loads page for cookies, generates session) ───
    let client = MsDownloadClient::init(page_url, cli.cookie, cli.debug).await?;

    eprintln!("{} Fetching available languages…", cyan.apply_to("➜"));

    // ── 3. Fetch SKUs ──────────────────────────────────────────────────────
    let skus = client.get_skus(edition_id).await?;

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
    eprintln!(
        "{} Requesting download link (valid for 24 hours)…",
        cyan.apply_to("➜")
    );
    let dl = client.get_download_links(&sku.id).await?;
    let option = &dl.product_download_options[0];

    if let Some(exp) = &dl.download_expiration_datetime {
        eprintln!("  Link expires: {exp}");
    }

    if cli.url_only {
        println!("{}", option.uri);
        return Ok(());
    }

    // ── 6. Download ────────────────────────────────────────────────────────
    let filename = sku
        .friendly_file_names
        .first()
        .cloned()
        .unwrap_or_else(|| filename_from_url(&option.uri));

    let dest = cli
        .output
        .unwrap_or_else(|| PathBuf::from("."))
        .join(&filename);

    eprintln!(
        "{} Downloading {} → {}",
        green.apply_to("⬇"),
        bold.apply_to(&option.name),
        dest.display()
    );

    client.download_file(&option.uri, &dest).await?;

    eprintln!(
        "\n{} Saved to {}",
        green.apply_to("✔"),
        bold.apply_to(dest.display())
    );
    Ok(())
}
