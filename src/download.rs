use anyhow::{bail, Context, Result};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::header;
use sha2::Digest;
use tokio::io::AsyncWriteExt;

use crate::client::MsDownloadClient;
use crate::types::Cancelled;

impl MsDownloadClient {
    /// Download a file to disk (native only — not available on wasm).
    pub async fn download_file(
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

            let mut hasher: Option<sha2::Sha256> = if compute_hash {
                Some(Digest::new())
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
                                if let Some(h) = &mut hasher { Digest::update(h, &chunk); }
                                file.write_all(&chunk).await?;
                                pb.inc(chunk.len() as u64);
                            }
                            Some(Err(e)) => bail!("Error reading download stream: {e}"),
                            None => break,
                        }
                    }
                }
            }

            inline_hash = hasher.map(|h| hex::encode(Digest::finalize(h)));
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

pub async fn compute_file_hash(path: &std::path::Path) -> Result<String> {
    use tokio::io::AsyncReadExt;

    let mut file = tokio::fs::File::open(path)
        .await
        .with_context(|| format!("Cannot open {} for hashing", path.display()))?;
    let mut hasher = sha2::Sha256::new();
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
