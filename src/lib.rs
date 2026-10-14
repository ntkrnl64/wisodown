mod client;
#[cfg(not(target_arch = "wasm32"))]
mod download;
mod edition;
#[cfg(not(target_arch = "wasm32"))]
mod legacy;
mod parse;
mod types;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub use client::MsDownloadClient;
#[cfg(not(target_arch = "wasm32"))]
pub use download::{compute_file_hash, compute_file_md5};
pub use edition::*;
#[cfg(not(target_arch = "wasm32"))]
pub use legacy::*;
pub use parse::{lookup_page_hash, parse_page_hashes};
pub use types::{Cancelled, DownloadOption, DownloadResponse, Sku};
