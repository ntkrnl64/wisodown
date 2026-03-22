# Usage Reference

## Synopsis

```
wisodown [OPTIONS]
```

Run without options for the interactive wizard.

## Options

| Flag | Description |
| ---- | ----------- |
| `-e, --edition <ALIAS>` | Named edition: `x64`, `arm64`, or `win10` |
| `--edition-id <N>` | Raw numeric `ProductEditionId` (advanced) |
| `--page-url <URL>` | Override cookie-acquisition page (requires `--edition-id`) |
| `-l, --language <LANG>` | Language name, e.g. `English`, `French`, `Japanese`. Case-insensitive. |
| `-o, --output <DIR>` | Output directory (default: current directory) |
| `--threads <N>` | Parallel connections for downloading (default: `4`) |
| `--url-only` | Print the download URL without downloading |
| `--list-languages` | List available languages for the chosen edition and exit |
| `--cookie <STR>` | Browser cookie string for manual anti-bot bypass |
| `--verify <SHA256>` | Assert an expected SHA-256 hex digest (overrides page hash). Mutually exclusive with `--no-verify`. |
| `--no-verify` | Skip SHA-256 fetching, computation, and verification entirely. In multi-threaded mode also skips the re-read hash pass. |
| `--debug` | Print raw API requests and responses to stderr |

## Edition aliases

| `--edition` | Windows version |
| ----------- | --------------- |
| `x64` | Windows 11, 64-bit (x86-64) |
| `arm64` | Windows 11, ARM 64-bit |
| `win10` | Windows 10, multi-edition (Home/Pro, 32-bit + 64-bit) |

Aliases `win11`, `win11-x64`, `win11-arm64`, `amd64`, `aarch64` are also accepted.

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

## Parallel downloading

By default `wisodown` uses **4 parallel connections** via HTTP Range requests, which typically 2–4× faster than a single connection on high-bandwidth links.

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

## List available languages

```sh
wisodown --edition win10 --list-languages
wisodown --edition x64 --list-languages
```

Output:

```
Available languages:
  Arabic                         (id: 1234, file: Win11_24H2_Arabic_x64.iso)
  English                        (id: 5585, file: Win11_24H2_English_x64.iso)
  French                         (id: 5586, file: Win11_24H2_French_x64.iso)
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
