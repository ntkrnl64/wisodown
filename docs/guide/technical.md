# Technical Reference

This document describes the internals of **wisodown**: its architecture, module
layout, the Microsoft API flow it reproduces, build prerequisites, Linux
portability, and the helper scripts. It is the companion to
[How It Works](/guide/how-it-works), which focuses on the runtime protocol.

> A Spanish translation is available at
> [Referencia técnica](/guide/technical.es).

---

## 1. Overview

`wisodown` is a Rust workspace with three usage modes built from one core
library:

| Mode | Target | Entry point |
| ---- | ------ | ----------- |
| CLI | native | `src/main.rs` → `wisodown` |
| Self-hosted API + web UI | native (`server` feature) | `src/server.rs` → `wisodown-server` |
| Browser / WASM | `wasm32` | `src/wasm.rs` → `windows_iso_downloader` (cdylib) |

All three share the same `MsDownloadClient` implementation, so the Sentinel
bypass, SKU lookup, link generation, and hash scraping logic live in exactly one
place.

```
src/
├── lib.rs        # public API surface + feature gating
├── main.rs       # CLI (clap): wizard, flags, progress, hashing
├── server.rs     # axum HTTP API + embedded frontend + SQLite link cache
├── wasm.rs       # wasm-bindgen wrapper
├── client.rs     # MsDownloadClient: session, fingerprints, API calls
├── download.rs   # streaming + parallel (Range) download, SHA-256
├── edition.rs    # edition aliases → ProductEditionId + page URL
├── parse.rs      # HTML/JS scraping of config, SDS bundle, hash table
└── types.rs      # serde models + shared config
```

---

## 2. Crate layout

`Cargo.toml` defines:

- **Library** `windows_iso_downloader` (`src/lib.rs`), `crate-type = ["lib", "cdylib"]`.
- **Binary** `wisodown` (`src/main.rs`) — the CLI.
- **Binary** `wisodown-server` (`src/server.rs`), gated behind `required-features = ["server"]`.

Dependencies are split by target so the same crate compiles for native and
`wasm32`:

- **Core (all targets):** `reqwest` (json only), `serde`, `serde_json`, `uuid`,
  `anyhow`, `sha2`, `hex`.
- **Native only:** `tokio`, `clap`, `dialoguer`, `console`, `indicatif`,
  `futures-util`, `bytes`, `axum`, `tower-http`, `rust-embed`, `mime_guess`,
  `rusqlite` (bundled), `time`.
- **WASM only:** `js-sys`, `wasm-bindgen`, `wasm-bindgen-futures`,
  `serde-wasm-bindgen`.

### TLS backend

`reqwest` is configured with `default-features = false` and the `rustls-tls`
feature. This is deliberate:

- It removes the dependency on the system OpenSSL (`native-tls` → `openssl-sys`).
- Certificate roots are bundled (`webpki-roots`), so the binary behaves
  identically on distros without a usable system CA store.

As a result, on Linux there is **no `libssl-dev` / OpenSSL requirement**. The
only native build dependency left is a C compiler, needed by `rusqlite` with the
`bundled` feature (used exclusively by `wisodown-server`).

---

## 3. Module reference

### `client.rs` — `MsDownloadClient`

| Method | Purpose |
| ------ | ------- |
| `init(page_url, cookie, debug)` | Creates the HTTP client, loads the page, extracts dynamic config, runs both fingerprint flows. |
| `get_skus(edition_id)` | Calls `getSkuInformationByProductEdition`, returns `Vec<Sku>`. |
| `get_download_links(sku_id)` | Calls `GetProductDownloadLinksBySku`, returns `DownloadResponse`. |
| `fetch_page_hashes()` | Scrapes the SHA-256 table from the page. |
| `download_file(url, path, threads, hash)` | Native-only; delegates to `download.rs::download_url`. |

The client uses a fixed Chrome-on-Windows `User-Agent` for the *page* request.
That is intentional — Microsoft redirects Windows UAs to the Media Creation Tool
— and is unrelated to the host OS.

### `download.rs`

Single connection streams to a `.part` file and hashes on the fly. With
`--threads > 1` and a server that advertises `Accept-Ranges: bytes`, the file is
pre-allocated with `set_len`, split into N equal byte ranges, and each range is
fetched in its own tokio task with its own file handle (`seek` + `BufWriter`).
Hashing then happens in a second sequential pass. `tokio::select!` races the
download against `ctrl_c()` for clean cancellation.

### `edition.rs`

Maps user aliases (`x64`, `arm64`, `win10`, `win11-cn-home`, `win11-cn-pro`, …)
to a `(ProductEditionId, page_url)` pair. Also hosts `find_sku_by_language` and
`filename_from_url`.

The aliases resolve to the current Microsoft products (verified live against the
Microsoft API):

| Alias | `ProductEditionId` | Page | Version | Architecture | ISO filename |
| ----- | ------------------ | ---- | ------- | ------------ | ------------ |
| `x64` | `3321` | `software-download/windows11` | Windows 11 **25H2** | x64 | `Win11_25H2_<Lang>_x64_v2.iso` |
| `arm64` | `3324` | `software-download/windows11arm64` | Windows 11 **25H2** | Arm64 | `Win11_25H2_<Lang>_Arm64_v2.iso` |
| `win10` | `2618` | `software-download/windows10ISO` | Windows 10 **22H2** | x86 + x64 | `Win10_22H2_<Lang>_x32v1.iso` / `…_x64v1.iso` |
| `win11-cn-home` | `3322` | `software-download/windows11` | Windows 11 **25H2** Home China | x64 | Chinese (Simplified) only |
| `win11-cn-pro` | `3323` | `software-download/windows11` | Windows 11 **25H2** Pro China | x64 | Chinese (Simplified) only |

Microsoft only serves the **latest** feature update per product: Windows 11
**25H2** and Windows 10 **22H2**. Consumer ISOs are **multi-edition** (Home, Pro,
Education, …) — the product key selects the edition at install time. There is no
32-bit Windows 11 media, and no LTSC/IoT media on the public pages.

`win10` is the only alias that returns **two** download options (32-bit `x32` and
64-bit `x64`); the CLI shows a `Select download` prompt whenever more than one
option comes back (`x64`/`arm64` return a single option). 38 languages are
available for Windows 10 and for Windows 11 x64/Arm64.

### `legacy.rs`

Bridges the **Windows 7 / 8 / 8.1** editions to **archive.org**, since Microsoft
no longer publishes those consumer ISOs (its `windows7ISO` page only shows an
end-of-support notice, and `GetProductDownloadLinksBySku` only knows current
products).

- A curated table (`LEGACY_EDITIONS`) maps the aliases to archive.org item
  identifiers: `win7` → `win7-ult-sp1-english`, `win8` →
  `Win8_RTM_MSDN_English`, `win8.1` → `win-8.1`.
- `collect_isos()` queries `https://archive.org/metadata/<item>` at runtime and
  returns every `.iso` file with its size and MD5 (the file list therefore stays
  in sync with archive.org).
- `LegacyIso::url()` builds `https://archive.org/download/<item>/<file>` (path
  segments percent-encoded).
- `language_from_filename()` / `arch_from_filename()` derive a human label from
  the filename (`Win8.1_<Lang>_x{32,64}.iso` or the ISO-639 prefix in
  `en_windows_8_x64_…iso`); `filter_by_language()` does an exact-then-substring
  match. Spanish (`es`) is supported among the two-letter language codes.
- Downloads reuse `download.rs` (`download_url`) with the same multi-threaded
  range machinery, then verify the archive.org **MD5** (`md-5` crate).

The CLI (`main.rs::run_legacy`) handles `win7`/`win8`/`win8.1` before the
Microsoft flow; the server (`server.rs`) exposes them through the same
`/api/resolve`, `/api/skus`, `/api/links` and `/api/hashes` endpoints.

### `parse.rs`

Pure string parsing — no HTML dependency:

- `extract_hidden_input` — reads `<input id="…" value="…">` (e.g. `endpoint-svc`).
- `extract_sds_js_url` — finds the SDS bundle `<script src>` from the page.
- `extract_from_sds_js` — pulls `profile`, `instanceId`, `orgId` from the bundle.
- `parse_page_hashes` / `lookup_page_hash` — build and query the SHA-256 table,
  tolerant of the `FriendlyFileNames` field being absent.

### `types.rs`

Serde models. Note `Sku::friendly_file_names` carries `#[serde(default)]`
because Microsoft's current API no longer returns `FriendlyFileNames`; the code
falls back to `filename_from_url` when it is absent.

### `server.rs` (feature `server`)

`axum` router exposing `/api/resolve`, `/api/skus`, `/api/links`, `/api/hashes`,
serving the embedded frontend (`rust-embed`, folder `frontend/dist`) with SPA
fallback. `/api/links` responses are cached in SQLite (`rusqlite`) keyed by
`(edition_id, language_key)` with a TTL derived from Microsoft's expiration
timestamp (or `--cache-ttl`).

The `win7`/`win8`/`win8.1` editions are handled by `legacy.rs`: `/api/resolve`
returns a `legacy:<edition>` id and the archive.org details URL, `/api/skus`
groups the ISOs into language SKUs, `/api/links` returns the archive.org
downloads (with MD5s in `hashes`), and `/api/hashes` returns filename → MD5.
These paths are not cached.

---

## 4. Microsoft API flow

```
init()
  ├─ GET page_url                         → httpOnly session cookies
  ├─ parse endpoint-svc + SDS bundle      → api_base, profile, instanceId, orgId
  ├─ GET vlscppe.microsoft.com/fp/tags.js → ThreatMetrix cookies
  ├─ GET vlscppe.microsoft.com/tags       → ThreatMetrix cookies
  ├─ GET ov-df.microsoft.com/mdt.js       → per-session `ticks`
  └─ GET ov-df.microsoft.com/?…           → fptctx2 + MUID cookies
get_skus()
  └─ GET <api_base>/getskuinformationbyproductedition
get_download_links()
  └─ GET <api_base>/GetProductDownloadLinksBySku → time-limited CDN URLs
fetch_page_hashes()
  └─ GET page_url → parse <td> hash table
download_file()
  ├─ HEAD url                             → Content-Length + Accept-Ranges
  └─ GET url (+ Range headers)            → .part file → rename → SHA-256
```

Sentinel protects the API endpoints; `fptctx2` and `MUID` must be present or the
request is rejected with `ErrorSettings.SentinelReject`. See
[How It Works](/guide/how-it-works) for the full protocol details.

---

## 5. Build prerequisites

| Requirement | Needed by | Notes |
| ----------- | --------- | ----- |
| Rust stable 1.75+ | everything | `rustup` recommended |
| C compiler (`cc`/`gcc`/`clang`) | `wisodown-server` | bundled SQLite via `rusqlite` |
| OpenSSL | — | **not required** (rustls) |

```sh
# CLI only
cargo build --release

# CLI + server
cargo build --release --features server
```

### Cross-compiling to WASM

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
wasm-pack build --target web
```

---

## 6. Linux portability notes

The codebase is already cross-platform. Concretely:

- **No OS-specific code.** There is no `#[cfg(windows)]`, no `winapi`/
  `windows-sys`, no `std::os::windows`, no `cmd.exe`, and no backslash-separated
  paths. Paths use `std::path::Path` / `PathBuf` throughout.
- **TLS is self-contained.** `rustls` + `webpki-roots` avoids the OpenSSL system
  dependency (see §2).
- **Signals.** Cancellation uses `tokio::signal::ctrl_c()`, which works on Unix
  and Windows.
- **Terminal UI.** `dialoguer`/`console`/`indicatif` are cross-platform.
- **Scripts.** `scripts/get-iso.sh` is Bash and already Linux/macOS-native;
  `scripts/get-iso.ps1` is written to run under PowerShell 7 (`pwsh`) on Linux
  and macOS as well as Windows.
- **CI.** `.github/workflows/ci.yml` builds on Ubuntu, macOS, and Windows to
  prevent regressions.

Build verified on Linux x86-64 (produces a native
`ELF 64-bit LSB pie executable`).

---

## 7. Helper scripts

### `scripts/get-iso.sh` (Linux / macOS)

Bash script that talks to a `wisodown-server`. Requires `curl` and `jq`. It
defaults to `http://localhost:3000`; override with `-a <url>` or
`WISODOWN_API=<url>`.

```sh
./scripts/get-iso.sh -e x64 -l English          # print link
./scripts/get-iso.sh -e arm64 -l Japanese -d    # download
```

### `scripts/get-iso.ps1` (Windows / Linux / macOS)

PowerShell 7 script. On Linux/macOS:

```sh
chmod +x scripts/get-iso.ps1
./scripts/get-iso.ps1 -Edition x64 -Language English
# or
pwsh ./scripts/get-iso.ps1 -Edition x64 -Language English -Download
```

Curl-qualified language names are URL-encoded with `[uri]::EscapeDataString`,
numeric prompts are validated with `[int]::TryParse`, and responses are coerced
to arrays so single-element cases behave identically across platforms.

---

## 8. Self-hosted server

```sh
cargo run --release --features server -- --port 3000 --host 0.0.0.0
```

| Route | Description |
| ----- | ----------- |
| `GET /api/resolve` | `?edition=` → `{ editionId, pageUrl }` |
| `GET /api/skus` | `?edition=` → language list |
| `GET /api/links` | `?edition=&language=` → download URLs + hashes (SQLite cached) |
| `GET /api/hashes` | `?edition=` → SHA-256 table |
| `GET /*` | embedded SPA (`frontend/dist`) |

CORS is configured with repeatable `--cors-origin` flags (`*` allows all).
Graceful shutdown is triggered on Ctrl+C.

---

## 9. CI & releases

- **CI** (`ci.yml`): on every push to `main` and on pull requests, runs
  `cargo check`/`build` on Ubuntu, macOS, and Windows, with and without the
  `server` feature.
- **Release** (`release.yml`): a matrix on Ubuntu / macOS / Windows (x86_64 and
  i686) builds and publishes per-platform archives —
  `wisodown-linux-x86_64.zip`, `wisodown-macos-arm64.zip`,
  `wisodown-windows-x86_64.zip` and `wisodown-windows-i686.zip`. Each archive
  contains both binaries, the EN/ES READMEs, the licenses, the helper scripts and
  the technical guides. `workflow_dispatch` accepts an optional `tag` input to
  (re)attach assets to an existing release.

---

## 10. Troubleshooting

| Symptom | Cause / fix |
| ------- | ----------- |
| `missing field 'FriendlyFileNames'` | Fixed with `#[serde(default)]`; ensure you're on `v0.1.3`+. |
| `ErrorSettings.SentinelReject` | Too many requests from your IP. Wait, use a VPN, or pass `--cookie`. |
| Build fails on missing `openssl` | You're on an old build using `native-tls`; upgrade so `rustls-tls` is used. |
| `cc: command not found` building server | Install a C compiler (needed by bundled SQLite). |
| No download link returned | SKU invalid/expired; re-run and/or supply `--cookie`. |
