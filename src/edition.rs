use anyhow::{bail, Result};

use crate::types::Sku;

// ── Known edition constants ────────────────────────────────────────────────
pub const PAGE_URL_WIN10: &str = "https://www.microsoft.com/en-us/software-download/windows10ISO";
pub const PAGE_URL_X64: &str = "https://www.microsoft.com/en-us/software-download/windows11";
pub const PAGE_URL_ARM64: &str = "https://www.microsoft.com/en-us/software-download/windows11arm64";

pub const EDITION_WIN10: &str = "2618";
pub const EDITION_X64: &str = "3321";
pub const EDITION_ARM64: &str = "3324";
pub const EDITION_CN_HOME: &str = "3322";
pub const EDITION_CN_PRO: &str = "3323";

pub fn resolve_edition(input: &str) -> Result<(&'static str, &'static str)> {
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

pub fn find_sku_by_language<'a>(skus: &'a [Sku], language: &str) -> Result<&'a Sku> {
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

pub fn filename_from_url(url: &str) -> String {
    url.split('?')
        .next()
        .and_then(|p| p.rsplit('/').next())
        .unwrap_or("windows.iso")
        .to_string()
}
