//! Archive.org fallback for **Windows 7 / 8 / 8.1**.
//!
//! Microsoft no longer publishes these consumer ISOs on its public download
//! pages (`windows7ISO` etc. were replaced by end-of-support notices), and the
//! `GetProductDownloadLinksBySku` endpoint only knows current products. For
//! these editions we therefore resolve against curated **archive.org** items
//! and read the file list from the archive.org metadata API at runtime, so the
//! available ISOs, sizes and MD5 hashes stay in sync.
//!
//! These are community mirrors, not Microsoft-operated servers. The origin
//! (`archive.org`) is kept explicit everywhere in the output.

use anyhow::{bail, Context, Result};
use serde::Deserialize;

// ── Curated catalogue ─────────────────────────────────────────────────────────

pub struct LegacyEdition {
    pub aliases: &'static [&'static str],
    pub label: &'static str,
    /// archive.org item identifiers, in preference order.
    pub items: &'static [&'static str],
}

pub const LEGACY_EDITIONS: &[LegacyEdition] = &[
    LegacyEdition {
        aliases: &["win7", "windows7", "7"],
        label: "Windows 7 Ultimate SP1 (multi-language)",
        items: &["win7-ult-sp1-english"],
    },
    LegacyEdition {
        aliases: &["win8", "windows8", "8"],
        label: "Windows 8 (RTM, English)",
        items: &["Win8_RTM_MSDN_English"],
    },
    LegacyEdition {
        aliases: &["win8.1", "win81", "windows8.1", "windows81", "8.1"],
        label: "Windows 8.1 (Home & Pro, multi-language)",
        items: &["win-8.1"],
    },
];

/// Resolve an edition alias to a legacy (archive.org) edition, if it matches.
pub fn resolve_legacy(input: &str) -> Option<&'static LegacyEdition> {
    let key = input.trim().to_ascii_lowercase();
    LEGACY_EDITIONS
        .iter()
        .find(|e| e.aliases.iter().any(|a| *a == key))
}

/// `true` when `input` is a legacy edition alias.
pub fn is_legacy(input: &str) -> bool {
    resolve_legacy(input).is_some()
}

// ── archive.org metadata ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ItemMetadata {
    #[serde(default)]
    files: Vec<ArchiveFile>,
}

#[derive(Debug, Clone, Deserialize)]
struct ArchiveFile {
    name: String,
    #[serde(default)]
    size: Option<String>,
    #[serde(default)]
    md5: Option<String>,
    #[serde(default)]
    sha1: Option<String>,
}

/// A concrete ISO hosted on archive.org.
#[derive(Debug, Clone)]
pub struct LegacyIso {
    pub item: String,
    pub name: String,
    pub size: Option<u64>,
    pub md5: Option<String>,
    #[allow(dead_code)]
    pub sha1: Option<String>,
}

impl LegacyIso {
    /// Direct download URL (`https://archive.org/download/<item>/<file>`).
    pub fn url(&self) -> String {
        format!(
            "https://archive.org/download/{}/{}",
            self.item,
            encode_path_segment(&self.name)
        )
    }

    /// Best-effort language name parsed from the filename.
    pub fn language(&self) -> Option<String> {
        language_from_filename(&self.name)
    }

    /// Best-effort architecture label parsed from the filename.
    pub fn arch(&self) -> Option<&'static str> {
        arch_from_filename(&self.name)
    }

    /// Human-readable label, e.g. `Win8.1_English_x64.iso — English, 64-bit`.
    pub fn label(&self) -> String {
        let mut tags = Vec::new();
        if let Some(l) = self.language() {
            tags.push(l);
        }
        if let Some(a) = self.arch() {
            tags.push(a.to_string());
        }
        if tags.is_empty() {
            self.name.clone()
        } else {
            format!("{}  ({})", self.name, tags.join(", "))
        }
    }
}

/// Fetch every `.iso` file for a legacy edition from the archive.org metadata
/// API, sorted by filename.
pub async fn collect_isos(
    http: &reqwest::Client,
    edition: &LegacyEdition,
) -> Result<Vec<LegacyIso>> {
    let mut out: Vec<LegacyIso> = Vec::new();

    for item in edition.items {
        let url = format!("https://archive.org/metadata/{item}");
        let meta: ItemMetadata = http
            .get(&url)
            .send()
            .await
            .with_context(|| format!("Failed to query archive.org metadata for '{item}'"))?
            .json()
            .await
            .with_context(|| format!("archive.org returned invalid metadata for '{item}'"))?;

        out.extend(
            meta.files
                .into_iter()
                .filter(|f| f.name.to_ascii_lowercase().ends_with(".iso"))
                .map(|f| LegacyIso {
                    item: (*item).to_string(),
                    name: f.name,
                    size: f.size.as_deref().and_then(|s| s.parse::<u64>().ok()),
                    md5: f.md5.filter(|s| !s.is_empty()),
                    sha1: f.sha1.filter(|s| !s.is_empty()),
                }),
        );
    }

    if out.is_empty() {
        bail!(
            "No ISO files found for {} on archive.org (the item may have moved).",
            edition.label
        );
    }

    out.sort_by_key(|a| a.name.to_lowercase());
    Ok(out)
}

/// Filter ISOs by a language query. An exact language match wins; otherwise we
/// fall back to a case-insensitive substring match on the language or filename
/// (so `English` still finds `EnglishUS`/`EnglishInternational`).
pub fn filter_by_language<'a>(isos: &'a [LegacyIso], language: &str) -> Vec<&'a LegacyIso> {
    let q = language.trim().to_ascii_lowercase();
    let exact: Vec<&LegacyIso> = isos
        .iter()
        .filter(|i| {
            i.language()
                .map(|l| l.to_ascii_lowercase() == q)
                .unwrap_or(false)
        })
        .collect();
    if !exact.is_empty() {
        return exact;
    }
    isos.iter()
        .filter(|i| {
            i.language()
                .map(|l| l.to_ascii_lowercase().contains(&q))
                .unwrap_or(false)
                || i.name.to_ascii_lowercase().contains(&q)
        })
        .collect()
}

// ── Downloads ─────────────────────────────────────────────────────────────────

/// Download an archive.org ISO using the shared download machinery.
pub async fn download_iso(
    http: &reqwest::Client,
    url: &str,
    temp_path: &std::path::Path,
    threads: usize,
    compute_hash: bool,
) -> Result<Option<String>> {
    crate::download::download_url(http, url, temp_path, threads, compute_hash).await
}

/// Compute the hex MD5 of a downloaded file.
pub async fn compute_md5(path: &std::path::Path) -> Result<String> {
    crate::download::compute_file_md5(path).await
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Format a byte count as a human-readable size (`2.4 GiB`), or `?` if unknown.
pub fn format_size(bytes: Option<u64>) -> String {
    match bytes {
        Some(b) => {
            const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
            let mut v = b as f64;
            let mut i = 0;
            while v >= 1024.0 && i < UNITS.len() - 1 {
                v /= 1024.0;
                i += 1;
            }
            if i == 0 {
                format!("{b} B")
            } else {
                format!("{v:.1} {}", UNITS[i])
            }
        }
        None => "?".to_string(),
    }
}

fn arch_from_filename(name: &str) -> Option<&'static str> {
    let n = name.to_ascii_lowercase();
    if n.contains("x64") || n.contains("x86_64") || n.contains("amd64") || n.contains("64bit") {
        Some("64-bit")
    } else if n.contains("x32") || n.contains("x86") || n.contains("i386") || n.contains("32bit") {
        Some("32-bit")
    } else {
        None
    }
}

/// Parse a language name out of an archive.org filename. Handles the two shapes
/// used by our curated items:
///
/// * `Win8.1_<Language>_x64.iso`, `Win7_Ult_SP1_<Language>_x32.iso`
/// * `en_windows_8_x64_dvd_915440.iso` (ISO-639 two-letter prefix)
fn language_from_filename(name: &str) -> Option<String> {
    let stem = name.rsplit_once('.').map(|(a, _)| a).unwrap_or(name);
    let tokens: Vec<&str> = stem.split('_').collect();

    // Shape A: the token immediately before the architecture token.
    if let Some(i) = tokens.iter().position(|t| {
        matches!(
            t.to_ascii_lowercase().as_str(),
            "x32" | "x64" | "x86" | "x86_64" | "amd64"
        )
    }) {
        if i > 0 {
            let cand = tokens[i - 1];
            if cand.len() > 1 && !cand.chars().all(|c| c.is_ascii_digit()) && !is_junk_token(cand) {
                return Some(cand.to_string());
            }
        }
    }

    // Shape B: ISO-639 two-letter prefix (en_windows_8_x64_dvd_...).
    if let Some(code) = tokens.first() {
        if code.len() == 2 {
            if let Some(lang) = two_letter_language(&code.to_ascii_lowercase()) {
                return Some(lang.to_string());
            }
        }
    }

    None
}

fn is_junk_token(t: &str) -> bool {
    matches!(
        t.to_ascii_lowercase().as_str(),
        "ult"
            | "sp0"
            | "sp1"
            | "pro"
            | "home"
            | "core"
            | "le"
            | "n"
            | "k"
            | "kn"
            | "dvd"
            | "vl"
            | "rtm"
            | "with"
            | "update"
            | "windows"
            | "win"
            | "x"
    )
}

fn two_letter_language(code: &str) -> Option<&'static str> {
    Some(match code {
        "ar" => "Arabic",
        "bg" => "Bulgarian",
        "cs" => "Czech",
        "da" => "Danish",
        "de" => "German",
        "el" => "Greek",
        "en" => "English",
        "es" => "Spanish",
        "et" => "Estonian",
        "fi" => "Finnish",
        "fr" => "French",
        "he" => "Hebrew",
        "hr" => "Croatian",
        "hu" => "Hungarian",
        "it" => "Italian",
        "ja" => "Japanese",
        "ko" => "Korean",
        "lt" => "Lithuanian",
        "lv" => "Latvian",
        "nb" => "Norwegian",
        "nl" => "Dutch",
        "pl" => "Polish",
        "pt" => "Portuguese",
        "ro" => "Romanian",
        "ru" => "Russian",
        "sk" => "Slovak",
        "sl" => "Slovenian",
        "sv" => "Swedish",
        "th" => "Thai",
        "tr" => "Turkish",
        "uk" => "Ukrainian",
        "zh" => "Chinese",
        _ => return None,
    })
}

/// Percent-encode a path segment, keeping unreserved characters and parentheses
/// (which are legal in a URL path and used by several archive.org filenames).
fn encode_path_segment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'(' | b')' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_aliases() {
        assert_eq!(
            resolve_legacy("win7").unwrap().label,
            LEGACY_EDITIONS[0].label
        );
        assert_eq!(
            resolve_legacy("Win8").unwrap().label,
            LEGACY_EDITIONS[1].label
        );
        assert_eq!(
            resolve_legacy("WIN8.1").unwrap().label,
            LEGACY_EDITIONS[2].label
        );
        assert!(resolve_legacy("win10").is_none());
        assert!(is_legacy("8.1"));
    }

    #[test]
    fn parses_arch_and_language() {
        assert_eq!(
            arch_from_filename("Win7_Ult_SP1_English_x32.iso"),
            Some("32-bit")
        );
        assert_eq!(arch_from_filename("Win8.1_Spanish_x64.iso"), Some("64-bit"));
        assert_eq!(
            arch_from_filename("en_windows_8_x86_dvd.iso"),
            Some("32-bit")
        );
        assert_eq!(
            language_from_filename("Win8.1_Chinese(Simplified)_x64.iso").as_deref(),
            Some("Chinese(Simplified)")
        );
        assert_eq!(
            language_from_filename("Win7_Ult_SP1_English_x32.iso").as_deref(),
            Some("English")
        );
        assert_eq!(
            language_from_filename("en_windows_8_x64_dvd_915440.iso").as_deref(),
            Some("English")
        );
    }

    #[test]
    fn builds_urls() {
        let iso = LegacyIso {
            item: "win-8.1".into(),
            name: "Win8.1_Chinese(Simplified)_x64.iso".into(),
            size: None,
            md5: None,
            sha1: None,
        };
        assert_eq!(
            iso.url(),
            "https://archive.org/download/win-8.1/Win8.1_Chinese(Simplified)_x64.iso"
        );
    }

    #[test]
    fn formats_sizes() {
        assert_eq!(format_size(Some(3320903680)), "3.1 GiB");
        assert_eq!(format_size(None), "?");
    }
}
