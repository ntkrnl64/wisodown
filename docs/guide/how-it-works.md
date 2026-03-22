# How It Works

This page describes the full lifecycle of a `wisodown` run, from session setup through file delivery.

## Overview

```
wisodown
  │
  ├─ 1. Session init    — load page, acquire httpOnly cookies
  ├─ 2. Dynamic config  — extract profile/instanceId/orgId from SDS JS bundle
  ├─ 3. Fingerprint     — VLSC (ThreatMetrix) + ov-df (fptctx2 + MUID)
  ├─ 4. SKU list        — getSkuInformationByProductEdition
  ├─ 5. Download link   — GetProductDownloadLinksBySku
  ├─ 6. Page hashes     — scrape SHA-256 table from Microsoft's page
  └─ 7. Download        — parallel Range requests → .part file → rename
```

---

## 1. Session initialization

`wisodown` creates a random UUID v4 **session ID** — the same thing the browser-side JavaScript generates with `crypto.randomUUID()`. This ID ties all API requests in a session together.

It then performs a `GET` on the edition's download page (e.g. `/software-download/windows11`) using a Mac/Chrome `User-Agent` string. This is required because:

- On a Windows User-Agent the page redirects to the Media Creation Tool instead of showing the ISO download form.
- The `GET` causes Microsoft's servers to set several `httpOnly` session cookies via `Set-Cookie` response headers. These cookies are stored in reqwest's cookie jar and sent automatically on all subsequent requests to `microsoft.com`.

---

## 2. Dynamic configuration

The download page embeds a JavaScript bundle (`sds/components/content/sdsbase/…/site.ACSHASH*.min.js`) that contains:

- **`profile`** — API profile token (e.g. `606624d44113`)
- **`instanceId`** — ov-df fingerprint instance UUID
- **`orgId`** — ThreatMetrix organisation ID (e.g. `y6jn8c31`)

`wisodown` fetches the page HTML, locates the `<script src="…sdsbase…">` tag, fetches the JS bundle, and extracts all three values dynamically. It also reads the `endpoint-svc` hidden `<input>` for the API base URL. This means new values published by Microsoft are picked up automatically without a tool update.

---

## 3. Browser fingerprint handshake

Microsoft's download API is protected by **Sentinel**, a bot-detection layer. The browser page runs **two** independent fingerprinting systems:

### VLSC / ThreatMetrix

1. Loads `https://vlscppe.microsoft.com/fp/tags.js?org_id=<orgId>&session_id=<uuid>` — a profiling script that sets tracking cookies.
2. Hits the iframe endpoint `https://vlscppe.microsoft.com/tags?org_id=<orgId>&session_id=<uuid>` which sets additional cookies.

### ov-df (Device Fingerprint)

1. Loads `https://ov-df.microsoft.com/mdt.js?instanceId=<id>&pageId=si&session_id=<uuid>` to obtain a per-session `ticks` token.
2. Loads a fingerprint iframe URL constructed from that token, which responds with `Set-Cookie` headers for:
   - `fptctx2` — Sentinel's session proof token
   - `MUID` — Microsoft's user ID cookie scoped to `.microsoft.com`

Without cookies from **both** systems the API returns a Sentinel rejection. `wisodown` replicates both flows sequentially.

---

## 4. SKU list — `getSkuInformationByProductEdition`

```
GET <api_base>/getskuinformationbyproductedition
    ?profile=<profile>
    &ProductEditionId=3321        ← edition (3321=Win11 x64, 3324=ARM64, 2618=Win10,
                                              3322=Win11 Home China, 3323=Win11 Pro China)
    &Locale=en-US
    &sessionID=<uuid>
```

Returns a JSON array of **SKUs** — one per language. Each SKU has:
- `Id` — used in the next API call
- `Language` / `LocalizedLanguage` — display names
- `FriendlyFileNames` — the ISO filename Microsoft will serve

The `Referer` header must match the download page URL, and the `fptctx2`/`MUID` cookies must be present or Sentinel blocks the request.

---

## 5. Download link — `GetProductDownloadLinksBySku`

Before this call, `wisodown` re-fetches the download page to refresh the short-lived **`CAS_PROGRAM`** cookie (~8 second TTL) that Sentinel requires on this endpoint.

```
GET <api_base>/GetProductDownloadLinksBySku
    ?profile=<profile>
    &SKU=<sku_id>
    &Locale=en-US
    &sessionID=<uuid>
```

Returns a list of **download options** with time-limited (~24h) CDN URLs. For Windows 10 this includes both 32-bit and 64-bit options; for Windows 11 there is typically one option per language.

---

## 6. Page hash scraping

Microsoft publishes SHA-256 hashes for every ISO on the download page in a plain HTML table:

```html
<td>English 64-bit</td>
<td>768984706B909479417B2368438909440F2967FF05C6A9195ED2667254E465E3</td>
```

`wisodown` fetches the page HTML again (separate from the session init visit) and parses these pairs using a simple walk: it finds adjacent `<td>` pairs where the second cell is exactly 64 hex characters, and builds a map from lowercased language+arch keys to lowercased hashes.

The matching logic:
1. Infers `64-bit` or `32-bit` from the download option name.
2. Tries an exact key match (`"english 64-bit"`).
3. Falls back to prefix match to handle variants like `"English International"`.

This step is skipped when `--no-verify` is passed.

---

## 7. Parallel download

### Temp file approach

The ISO is always written to a `.part` file in the same directory as the final destination (e.g. `Win11_English_x64.iso.part`). On completion it is atomically renamed to the final path. This ensures:

- A partial download never appears as a complete file at the destination.
- Ctrl+C or any error removes the `.part` file cleanly.
- An existing file at the destination is never overwritten until the new download is verified complete.

### Range requests (multi-threaded)

When `--threads N` is greater than 1 (default: 8):

1. A `HEAD` request checks `Accept-Ranges: bytes` and `Content-Length`. If either is missing or ranges are not supported, a single-connection fallback is used.
2. The file is **pre-allocated** on disk using `set_len(total)` to avoid fragmentation and ensure all offsets are writable.
3. The file range is split into N equal segments. Each segment is fetched with an HTTP `Range: bytes=start-end` header in its own tokio task.
4. Each task opens its own OS file handle (via `std::fs::File::open` + `seek`) pre-positioned at the correct offset, then wraps it in a `tokio::io::BufWriter`. Since each handle writes to a non-overlapping byte range, no locking is needed.
5. A shared `ProgressBar` is incremented by each task as chunks arrive.
6. Cancellation uses `tokio::select!` — the main task races the `join_all(tasks)` future against `tokio::signal::ctrl_c()`. On Ctrl+C, the `.part` file is removed and the process exits with code 130.

### SHA-256 computation

- **Single-threaded mode**: SHA-256 is computed on-the-fly in the streaming loop — free, no extra I/O.
- **Multi-threaded mode**: Because chunks arrive at arbitrary offsets, in-stream hashing would require reordering. Instead, after all tasks complete, the finished `.part` file is read sequentially in 8 MB chunks to compute the digest.
- **`--no-verify`**: The re-read pass in multi-threaded mode is skipped entirely.

---

## Error handling

| Situation | Behaviour |
| --------- | --------- |
| Ctrl+C during download | `.part` file deleted, exit code 130 |
| Download stream error | `.part` file deleted, error reported |
| Hash mismatch | File **kept** at destination, error reported (user can inspect it) |
| Server no Range support | Transparent fallback to single connection |
| API Sentinel block | Error with hint to use `--cookie` |
