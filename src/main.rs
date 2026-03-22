use anyhow::{Context, Result};
use clap::Parser;
use console::Style;
use dialoguer::{theme::ColorfulTheme, Select};
use std::path::PathBuf;
use windows_iso_downloader::*;

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
