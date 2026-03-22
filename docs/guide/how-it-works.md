# How It Works

This page describes the full lifecycle of a `wisodown` run, from session setup through file delivery.

## Overview

```
wisodown
  │
  ├─ 1. Session init   — load page, acquire httpOnly cookies
  ├─ 2. Fingerprint    — simulate browser DFP handshake (fptctx2 + MUID)
  ├─ 3. SKU list       — getSkuInformationByProductEdition
  ├─ 4. Download link  — GetProductDownloadLinksBySku
  ├─ 5. Page hashes    — scrape SHA-256 table from Microsoft's page
  └─ 6. Download       — parallel Range requests → .part file → rename
```

---

## 1. Session initialization

`wisodown` creates a random UUID v4 **session ID** — the same thing the browser-side JavaScript generates with `crypto.randomUUID()`. This ID ties all API requests in a session together.

It then performs a `GET` on the edition's download page (e.g. `/software-download/windows11`) using a Mac/Chrome `User-Agent` string. This is required because:

- On a Windows User-Agent the page redirects to the Media Creation Tool instead of showing the ISO download form.
- The `GET` causes Microsoft's servers to set several `httpOnly` session cookies via `Set-Cookie` response headers. These cookies are stored in reqwest's cookie jar and sent automatically on all subsequent requests to `microsoft.com`.

---

## 2. Browser fingerprint handshake

Microsoft's download API is protected by **Sentinel**, a bot-detection layer. The browser page runs JavaScript that:

1. Loads `https://ov-df.microsoft.com/mdt.js` to obtain a per-session `ticks` token.
2. Loads a fingerprint iframe URL constructed from that token, which responds with `Set-Cookie` headers for:
   - `fptctx2` — Sentinel's session proof token
   - `MUID` — Microsoft's user ID cookie scoped to `.microsoft.com`

Without these cookies the API returns an error. `wisodown` replicates this handshake:

1. Fetches `mdt.js` and extracts the `ticks` value from the `&w=HEX` parameter in the iframe URL embedded in the response.
2. Calls the fingerprint endpoint with that token, collecting the resulting cookies.

---

## 3. SKU list — `getSkuInformationByProductEdition`

```
GET /software-download-connector/api/getskuinformationbyproductedition
    ?profile=606624d44113
    &ProductEditionId=3321        ← edition (3321=Win11 x64, 3324=ARM64, 2618=Win10)
    &Locale=en-US
    &sessionID=<uuid>
```

Returns a JSON array of **SKUs** — one per language. Each SKU has:
- `Id` — used in the next API call
- `Language` / `LocalizedLanguage` — display names
- `FriendlyFileNames` — the ISO filename Microsoft will serve

The `Referer` header must match the download page URL, and the `fptctx2`/`MUID` cookies must be present or Sentinel blocks the request.

---

## 4. Download link — `GetProductDownloadLinksBySku`

Before this call, `wisodown` re-fetches the download page to refresh the short-lived **`CAS_PROGRAM`** cookie (~8 second TTL) that Sentinel requires on this endpoint.

```
GET /software-download-connector/api/GetProductDownloadLinksBySku
    ?profile=606624d44113
    &SKU=<sku_id>
    &Locale=en-US
    &sessionID=<uuid>
```

Returns a list of **download options** with time-limited (~24h) CDN URLs. For Windows 10 this includes both 32-bit and 64-bit options; for Windows 11 there is typically one option per language.

---

## 5. Page hash scraping

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

## 6. Parallel download

### Temp file approach

The ISO is always written to a `.part` file in the same directory as the final destination (e.g. `Win11_English_x64.iso.part`). On completion it is atomically renamed to the final path. This ensures:

- A partial download never appears as a complete file at the destination.
- Ctrl+C or any error removes the `.part` file cleanly.
- An existing file at the destination is never overwritten until the new download is verified complete.

### Range requests (multi-threaded)

When `--threads N` is greater than 1 (default: 4):

1. A `HEAD` request checks `Accept-Ranges: bytes` and `Content-Length`. If either is missing or ranges are not supported, a single-connection fallback is used.
2. The file is **pre-allocated** on disk using `set_len(total)` to avoid fragmentation and ensure all offsets are writable.
3. The file range is split into N equal segments. Each segment is fetched with an HTTP `Range: bytes=start-end` header in its own tokio task.
4. All tasks share a single `Arc<Mutex<tokio::fs::File>>`. Each task locks the mutex, seeks to its current write position, writes the chunk, and releases the lock. Since network I/O is orders of magnitude slower than local writes, mutex contention is minimal.
5. A shared `ProgressBar` is incremented by each task as chunks arrive.
6. Cancellation uses an `Arc<AtomicBool>` set by a background ctrl_c listener; each task checks it per chunk.

### SHA-256 computation

- **Single-threaded mode**: SHA-256 is computed on-the-fly in the streaming loop — free, no extra I/O.
- **Multi-threaded mode**: Because chunks arrive at arbitrary offsets, in-stream hashing would require reordering. Instead, after all tasks complete, the finished `.part` file is read sequentially in 4 MB chunks to compute the digest.
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
