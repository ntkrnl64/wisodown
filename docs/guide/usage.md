# Usage Reference

## Synopsis

```
wisodown [OPTIONS]
```

Run without options for the interactive wizard.

## Options

| Flag | Description |
| ---- | ----------- |
| `-e, --edition <ALIAS>` | Named edition: `x64`, `arm64`, `win10`, `win11-cn-home`, `win11-cn-pro`, plus the archive.org editions `win7`, `win8`, `win8.1` |
| `--edition-id <N>` | Raw numeric `ProductEditionId` (advanced) |
| `--page-url <URL>` | Override cookie-acquisition page (requires `--edition-id`) |
| `-l, --language <LANG>` | Language name, e.g. `English`, `French`, `Japanese`. Case-insensitive. For legacy editions, an exact match wins, else a substring match on the filename. |
| `-o, --output <DIR>` | Output directory (default: current directory) |
| `-t, --threads <N>` | Parallel connections for downloading (default: `8`) |
| `--url-only` | Print the download URL without downloading |
| `--list-languages` | List available languages for the chosen edition and exit (legacy editions list ISO files) |
| `--cookie <STR>` | Browser cookie string for manual anti-bot bypass |
| `--verify <SHA256>` | Assert an expected SHA-256 hex digest (overrides page hash). Mutually exclusive with `--no-verify`. |
| `--no-verify` | Skip SHA-256 fetching, computation, and verification entirely. In multi-threaded mode also skips the re-read hash pass. |
| `--debug` | Print raw API requests and responses to stderr |

## Edition aliases

| `--edition` | Windows version | Architecture | ISO filename |
| ----------- | --------------- | ------------ | ------------ |
| `x64` | Windows 11 **25H2** | x64 (64-bit) | `Win11_25H2_<Lang>_x64_v2.iso` |
| `arm64` | Windows 11 **25H2** | Arm64 | `Win11_25H2_<Lang>_Arm64_v2.iso` |
| `win10` | Windows 10 **22H2** | x86 + x64 (choose at prompt) | `Win10_22H2_<Lang>_x32v1.iso` / `…_x64v1.iso` |
| `win11-cn-home` | Windows 11 **25H2** Home (China) | x64 | Chinese (Simplified) only |
| `win11-cn-pro` | Windows 11 **25H2** Pro (China) | x64 | Chinese (Simplified) only |
| `win7` | Windows 7 **Ultimate SP1** *(archive.org)* | x86 + x64 | `Win7_Ult_SP1_English_x32.iso` / `Win7_Ult_SP1_Spanish_x32.iso` / `…_x64.iso` / `…_x64.es.iso` |
| `win8` | Windows 8 **RTM** *(archive.org)* | x86 + x64 | `en_windows_8_x86_dvd_915417.iso` / `…_x64_…iso` |
| `win8.1` | Windows 8.1 **Home/Pro** *(archive.org)* | x86 + x64 | `Win8.1_<Lang>_x32.iso` / `Win8.1_<Lang>_x64.iso` |

Microsoft serves only the latest feature update per product (Windows 11 **25H2**,
Windows 10 **22H2**). Consumer ISOs are **multi-edition** and 38 languages are
available for Windows 10 and Windows 11 x64/Arm64.

The `win7`, `win8` and `win8.1` editions are downloaded from **archive.org**
(Microsoft retired those consumer ISOs). `win8.1` is multi-language; `win7`/`win8`
support **English and Spanish** (Spanish ISOs are available via archive.org). Downloads
are verified against the **MD5** published by archive.org; `--verify` still accepts a
SHA-256.

Aliases `win11`, `win11-x64`, `win11-arm64`, `amd64`, `aarch64`, `cn-home`, `cn-pro` are also accepted. Legacy editions additionally accept `windows7`, `windows8`, `windows8.1`, `win81`, `windows81` and the bare `7`, `8`, `8.1`.

## Hash verification

SHA-256 hashes are **fetched automatically** from the Microsoft download page before each download and verified against the file as it streams. You don't need to do anything — the output will show:

```
➜ Fetching integrity hashes from Microsoft…
  ℹ Expected SHA-256: 768984706b909479…
...
  SHA-256: 768984706b909479…
✔ Hash verified
```

On a hash mismatch, `wisodown` prints an error and exits with a non-zero code — **the file is kept** at the destination so you can inspect or retry it manually.

To use your own expected hash instead of the page hash, pass `--verify`:

```sh
wisodown --edition x64 --language English \
  --verify 768984706b909479417b2368438909440f2967ff05c6a9195ed2667254e465e3
```

To skip verification entirely (also skips the re-read pass in multi-threaded mode):

```sh
wisodown --edition x64 --language English --no-verify
```

## Requirements

SHA-256 hashes are **fetched automatically** from the Microsoft download page before each download and verified against the file as it streams. You don't need to do anything — the output will show:

```
➜ Fetching integrity hashes from Microsoft…
  ℹ Expected SHA-256: 768984706b909479…
...
  SHA-256: 768984706b909479…
✔ Hash verified
```

On a hash mismatch, `wisodown` prints an error and exits with a non-zero code — **the file is kept** at the destination so you can inspect or retry it manually.

To use your own expected hash instead of the page hash, pass `--verify`:

```sh
wisodown --edition x64 --language English \
  --verify 768984706b909479417b2368438909440f2967ff05c6a9195ed2667254e465e3
```

To skip verification entirely (also skips the re-read pass in multi-threaded mode):

```sh
wisodown --edition x64 --language English --no-verify
```

## Parallel downloading

By default `wisodown` uses **8 parallel connections** via HTTP Range requests, which typically 2–4× faster than a single connection on high-bandwidth links.

```sh
# Use 8 threads
wisodown --edition x64 --language English --threads 8

# Single connection (disables parallel mode)
wisodown --edition x64 --language English --threads 1
```

If the server doesn't support Range requests, `wisodown` automatically falls back to a single connection regardless of `--threads`.

The download is written to a `.part` file in the same directory and renamed to the final filename only after completion. This means:

- Interrupted downloads never leave a partial file at the destination path.
- Ctrl+C removes the `.part` file immediately.

---

## Windows support lifecycle

Microsoft's Windows support policy affects which ISO versions you can safely use with this tool:

- **Windows 7** mainstream support ended **January 13, 2015**, extended support ended **January 14, 2020**. No longer receives security updates.
- **Windows 8** support ended **January 12, 2016**. No longer receives security updates.
- **Windows 8.1** support ended **January 10, 2018**. No longer receives security updates.
- **Windows 10** mainstream support ended **October 13, 2020**. Extended support ends **October 14, 2025**.
- **Windows 10 ESU** (Extended Security Updates) is available for Windows 10 Enterprise through **January 2028** (via Volume Licensing). ESU receives critical security updates even after extended support ends.
- **Root certificate rotation**: Starting in 2023, Microsoft began rotating trusted root certificates. Systems running Windows 7/8/8.1 **without ESU** may lose the ability to connect to Windows Update and some Microsoft services as old roots expire and new ones are not installed. Windows 10 with ESU receives the certificate updates. This may affect the tool's ability to acquire Microsoft API cookies on unpatched systems.

> **Recommendation**: Use Windows 10 with ESU if still required for legacy workflows, or migrate to a currently supported OS. The tool itself runs on any OS with Rust, but the Windows ISOs it downloads have the above lifecycle.

## Using raw edition IDs

Every ISO product on Microsoft's site has a numeric `ProductEditionId`. You can pass it directly to download editions that don't have a named alias:

```sh
wisodown --edition-id 2618 --language English
```

If the product lives on a different download page (needed for correct cookies), supply the page URL too:

```sh
wisodown --edition-id 2618 \
  --page-url https://www.microsoft.com/en-us/software-download/windows10ISO \
  --language English
```

## Legacy editions (Windows 7 / 8 / 8.1)

These are served from **archive.org**, not Microsoft:

```sh
# Windows 7 Ultimate SP1, 64-bit (Spanish)
wisodown --edition win7 --language Spanish

# Windows 7 Ultimate SP1, 64-bit (English)
wisodown --edition win7 --language English

# Windows 8 RTM, 32-bit (English)
wisodown --edition win8 --language English

# Windows 8 RTM, 64-bit (English)
wisodown --edition win8 --language English

# Windows 8.1 (multi-language; 32- and 64-bit available)
wisodown --edition win8.1 --language Spanish

# List every ISO available for an edition
wisodown --edition win8.1 --list-languages

# Print the archive.org URL only
wisodown --edition win7 --language English --url-only
```

The ISO list (names, sizes, MD5) is read from the archive.org metadata API at
runtime, so it stays current. Because several languages ship both a 32-bit and a
64-bit file, a language query may match more than one ISO — you'll get a picker
unless the match is unique. `win7` and `win8` support English and Spanish;
`win8.1` uses substring matching on the filename. Downloads are verified against
archive.org's MD5.

## List available languages

```sh
wisodown --edition win10 --list-languages
wisodown --edition x64 --list-languages
```

Output:

```
Available languages:
  Arabic                         (id: 20035, file: Win11_25H2_Arabic_x64_v2.iso)
  English                        (id: 20046, file: Win11_25H2_English_x64_v2.iso)
  French                         (id: 20047, file: Win11_25H2_French_x64_v2.iso)
  ...
```

## Get the URL only

Print the time-limited CDN URL without downloading the file:

```sh
wisodown --edition x64 --language English --url-only
```

The URL is valid for approximately 24 hours.

## Anti-bot cookies

Microsoft's API uses a Sentinel bot-detection layer. `wisodown` attempts to bypass it automatically by simulating the browser's fingerprinting handshake. If requests fail with API errors, you can supply real browser cookies as a fallback:

1. Open the download page in a browser
2. Open DevTools → Network → any request to `microsoft.com` → Request Headers → Cookie
3. Copy the full cookie string:

```sh
wisodown --edition x64 --language English \
  --cookie "MUID=abc123; fptctx2=xyz..."
```

## Ctrl+C

Press **Ctrl+C** at any time. The partial download file is deleted and the process exits cleanly with code `130`.

## Debug mode

```sh
wisodown --edition x64 --language English --debug
```

Prints session ID, all HTTP requests/responses, fingerprint cookie steps, and hash lookup results to stderr.
