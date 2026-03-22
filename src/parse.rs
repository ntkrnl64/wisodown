use crate::types::PageConfig;
use std::collections::HashMap;

/// Extract the value of a hidden `<input>` by its `id` attribute.
pub(crate) fn extract_hidden_input(html: &str, id: &str) -> Option<String> {
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
pub(crate) fn extract_sds_js_url(html: &str) -> Option<String> {
    let marker = "sdsbase/v1/sdsbase/clientlibs/site.";
    let mut search = 0;
    while let Some(rel) = html[search..].find(marker) {
        let marker_pos = search + rel;
        let tail = &html[marker_pos..html.len().min(marker_pos + 120)];
        if tail.contains(".min.js") {
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
pub(crate) fn extract_from_sds_js(js: &str) -> PageConfig {
    let mut cfg = PageConfig::default();

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

pub fn parse_page_hashes(html: &str) -> HashMap<String, String> {
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

pub fn lookup_page_hash(
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
