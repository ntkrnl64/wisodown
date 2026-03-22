mod client;
#[cfg(not(target_arch = "wasm32"))]
mod download;
mod edition;
mod parse;
mod types;

pub use client::MsDownloadClient;
#[cfg(not(target_arch = "wasm32"))]
pub use download::compute_file_hash;
pub use edition::*;
pub use parse::{lookup_page_hash, parse_page_hashes};
pub use types::{Cancelled, DownloadOption, DownloadResponse, Sku};
