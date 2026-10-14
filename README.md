# Windows ISO Downloader

**English** · [Español](README.es.md)

Download Windows 10 and Windows 11 ISOs directly from Microsoft's servers — plus Windows 7, 8 and 8.1 from archive.org — no Media Creation Tool, no browser required.

## Features

- Downloads ISOs straight from Microsoft's official CDN
- Supports **Windows 10** (multi-edition: Home/Pro, 32-bit and 64-bit)
- Supports **Windows 11 x64** and **Windows 11 arm64**
- Supports **Windows 7, 8 and 8.1** from **[archive.org](https://archive.org)** (Microsoft no longer publishes those consumer ISOs)
- **Raw edition ID support** — pass any numeric `ProductEditionId` from Microsoft's API
- **SHA-256 verification** — hash is computed while streaming and printed after download; pass `--verify` to fail on mismatch
- Interactive wizard when run without arguments
- All available languages supported
- Progress bar with speed and ETA
- `--url-only` mode to just print the download link

## What you can download

| `--edition` | Windows version | Architecture | ISO filename |
| ----------- | --------------- | ------------ | ------------ |
| `x64` | Windows 11 **25H2** | x64 (64-bit) | `Win11_25H2_<Lang>_x64_v2.iso` |
| `arm64` | Windows 11 **25H2** | Arm64 | `Win11_25H2_<Lang>_Arm64_v2.iso` |
| `win10` | Windows 10 **22H2** | x86 (32-bit) **and** x64 (64-bit) | `Win10_22H2_<Lang>_x32v1.iso` / `Win10_22H2_<Lang>_x64v1.iso` |
| `win11-cn-home` | Windows 11 **25H2** Home (China) | — | Chinese (Simplified) only |
| `win11-cn-pro` | Windows 11 **25H2** Pro (China) | — | Chinese (Simplified) only |
| `win7` | Windows 7 **Ultimate SP1** — archive.org | x86 (32-bit) **and** x64 (64-bit) | `Win7_Ult_SP1_English_x32.iso` / `Win7_Ult_SP1_Spanish_x32.iso` / `..._x64.iso` / `..._x64.es.iso` |
| `win8` | Windows 8 **RTM** — archive.org | x86 (32-bit) **and** x64 (64-bit) | `en_windows_8_x86_dvd_915417.iso` / `..._x64_...iso` |
| `win8.1` | Windows 8.1 **Home/Pro** — archive.org | x86 (32-bit) **and** x64 (64-bit) | `Win8.1_<Lang>_x32.iso` / `Win8.1_<Lang>_x64.iso` |

- ISOs are **multi-edition** (Home/Pro/Edu/…): your product key selects the edition during installation.
- **38 languages** are available for Windows 10/11 (x64 and Arm64).
- Windows 11 has **no 32-bit** ISO; Windows 10 lets you choose **32-bit or 64-bit**.
- Enterprise **LTSC / IoT** images are *not* on Microsoft's public download pages.
- Any other product: pass its numeric `ProductEditionId` with `--edition-id`.
- **Windows 7 / 8 / 8.1 are served from [archive.org](https://archive.org)**, not
  Microsoft: those consumer ISOs were retired from Microsoft's public pages.
  `win8.1` is **multi-language**; `win7`/`win8` support **English and Spanish**
  (Spanish ISOs are available via archive.org). The file list is read from the
  archive.org metadata API at runtime, and downloads are verified against the
  **MD5** published there (`--verify` still accepts a SHA-256).

## Requirements

- A Rust toolchain (`cargo`, stable 1.75+)
- A C compiler (`cc`/`gcc`/`clang`) — required by the bundled SQLite used by the optional server
- **No system OpenSSL needed**: TLS is provided by `rustls` with bundled roots, so the build is self-contained on any OS

## Windows support lifecycle

Microsoft's Windows support policy affects which ISO versions you can safely use with this tool:

- **Windows 7** mainstream support ended **January 13, 2015**, extended support ended **January 14, 2020**. No longer receives security updates.
- **Windows 8** support ended **January 12, 2016**. No longer receives security updates.
- **Windows 8.1** support ended **January 10, 2018**. No longer receives security updates.
- **Windows 10** mainstream support ended **October 13, 2020**. Extended support ends **October 14, 2025**.
- **Windows 10 ESU** (Extended Security Updates) is available for Windows 10 Enterprise through **January 2028** (via Volume Licensing). ESU receives critical security updates even after extended support ends.
- **Root certificate rotation**: Starting in 2023, Microsoft began rotating trusted root certificates. Systems running Windows 7/8/8.1 **without ESU** may lose the ability to connect to Windows Update and some Microsoft services as old roots expire and new ones are not installed. Windows 10 with ESU receives the certificate updates. This may affect the tool's ability to acquire Microsoft API cookies on unpatched systems.

> **Recommendation**: Use Windows 10 with ESU if still required for legacy workflows, or migrate to a currently supported OS. The tool itself runs on any OS with Rust, but the Windows ISOs it downloads have the above lifecycle.

## Install

```sh
cargo install --path .
# binary: wisodown
```

Or build manually:

```sh
cargo build --release
# binary at target/release/wisodown
```

### Building on Linux

The project is fully cross-platform and builds natively on Linux — there is no
Windows-specific code (`winapi`, `#[cfg(windows)]`, backslash paths, …). Only
the ISO *products* are Windows; the tool itself runs anywhere Rust runs.

```sh
git clone https://github.com/BENDER35/wisodown-lin
cd wisodown-lin
cargo build --release
# binary: target/release/wisodown
```

To also build the optional self-hosted server (needs a C compiler for bundled
SQLite):

```sh
cargo build --release --features server
# binary: target/release/wisodown-server
```

### Building on Windows

The tool builds natively on Windows too:

1. Install Rust from <https://rustup.rs> (`rustup-init.exe`). The installer uses
   the MSVC toolchain by default (requires the Visual Studio C++ Build Tools).
2. Build:

```powershell
git clone https://github.com/BENDER35/wisodown-lin
cd wisodown-lin
cargo build --release
# binary: target\release\wisodown.exe

# with the optional server (needs the C compiler for bundled SQLite):
cargo build --release --features server
# binary: target\release\wisodown-server.exe

# 32-bit (x86) build — needs the Visual Studio x86 C++ toolset:
rustup target add i686-pc-windows-msvc
cargo build --release --target i686-pc-windows-msvc
# binary: target\i686-pc-windows-msvc\release\wisodown.exe
```

**Host OS** support: **Windows 10 and Windows 11** — x64, ARM64 and 32-bit x86
(`i686`). Windows 7/8.1 are not supported by current Rust toolchains. (This is
about *where the binary runs* — the ISOs it downloads are Windows 7/8/8.1/10/11;
see [What you can download](#what-you-can-download).)

> **Prebuilt releases:** each release ships ready-to-run archives:
> `wisodown-linux-x86_64.zip`, `wisodown-macos-*.zip`,
> `wisodown-windows-x86_64.zip` (64-bit) and `wisodown-windows-i686.zip`
> (32-bit). You can also build from source, or use the helper script
> `scripts/get-iso.ps1` (no compilation needed).

### Helper scripts

| Script | Platform | Requirements |
| ------ | -------- | ------------ |
| `scripts/get-iso.sh` | Linux / macOS | `bash`, `curl`, `jq` |
| `scripts/get-iso.ps1` | Windows / Linux / macOS (PowerShell 7+) | `pwsh` |

Both talk to a running `wisodown-server` and can print a download link or
download the ISO directly. They default to `http://localhost:3000` (override with
`-a <url>` / `WISODOWN_API=<url>` for bash, `-ApiBase <url>` / `WISODOWN_API=<url>`
for PowerShell), so start a server first:

```sh
cargo run --release --features server -- --port 3000     # terminal 1
./scripts/get-iso.sh -e x64 -l English                   # terminal 2
```

On Linux the PowerShell script runs with
[`pwsh`](https://learn.microsoft.com/powershell/scripting/install/installing-powershell-on-linux):

```sh
pwsh ./scripts/get-iso.ps1 -Edition x64 -Language English
# or, after chmod +x:
./scripts/get-iso.ps1 -Edition x64 -Language English
```

## Usage

### Interactive wizard

```sh
wisodown
```

Prompts for Windows version, language, and (for Windows 10) 32-bit vs 64-bit.

### Named editions

```sh
# Windows 11 x64, English
wisodown --edition x64 --language English

# Windows 11 arm64, French
wisodown --edition arm64 --language French

# Windows 10, English (prompts for 32-bit vs 64-bit)
wisodown --edition win10 --language English

# Save to a specific directory
wisodown --edition x64 --language English --output ~/Downloads
```

### Legacy editions — Windows 7 / 8 / 8.1 (archive.org)

Microsoft no longer publishes the consumer Windows 7 / 8 / 8.1 ISOs, so these
editions are downloaded from **[archive.org](https://archive.org)** instead:

```sh
# Windows 7 Ultimate SP1, 64-bit (Spanish)
wisodown --edition win7 --language Spanish

# Windows 7 Ultimate SP1, 64-bit (English)
wisodown --edition win7 --language English

# Windows 8 RTM, 32-bit (English)
wisodown --edition win8 --language English

# Windows 8 RTM, 64-bit (English)
wisodown --edition win8 --language English

# Windows 8.1 — pick a language; both 32-bit and 64-bit are offered
wisodown --edition win8.1 --language Spanish

# See every ISO available for an edition
wisodown --edition win8.1 --list-languages

# Just print the archive.org URL
wisodown --edition win7 --language English --url-only
```

For `--language`, `win8.1` uses the language embedded in the filename
(`Spanish`, `German`, `Chinese(Simplified)`, …); an exact match wins, otherwise
it falls back to a substring match. `win7` and `win8` support **English and Spanish**
(Spanish ISOs are available via archive.org).

> These are **community mirrors** hosted by the Internet Archive, not
> Microsoft-operated servers. Downloads are verified against the MD5 published
> by archive.org.

### Raw edition IDs

Pass any numeric `ProductEditionId` directly, bypassing the named aliases:

```sh
wisodown --edition-id 2618 --language English
wisodown --edition-id 3321 --language Japanese
```

When using a custom edition ID for a product page other than Windows 11 x64,
supply the corresponding page URL so the correct cookies are acquired:

```sh
wisodown --edition-id 2618 \
         --page-url https://www.microsoft.com/en-us/software-download/windows10ISO \
         --language English
```

### `--edition` aliases

| Value   | Description                                                     |
| ------- | -------------------------------------------------------------- |
| `x64`   | Windows 11 25H2, 64-bit (x86-64)                                |
| `arm64` | Windows 11 25H2, Arm64                                          |
| `win10` | Windows 10 22H2, multi-edition (prompts 32-bit vs 64-bit)       |
| `win11-cn-home` | Windows 11 25H2 Home (China, Chinese Simplified only)   |
| `win11-cn-pro`  | Windows 11 25H2 Pro (China, Chinese Simplified only)    |
| `win7`          | Windows 7 Ultimate SP1, English and Spanish (**archive.org**)        |
| `win8`          | Windows 8 RTM, English (**archive.org**)                 |
| `win8.1`        | Windows 8.1 Home/Pro, multi-language (**archive.org**)   |

Aliases `win11`, `win11-x64`, `win11-arm64`, `amd64`, `aarch64`, `cn-home` and
`cn-pro` are also accepted. The legacy editions also accept `windows7`,
`windows8`, `windows8.1`, `win81`, `windows81` and the bare `7`, `8`, `8.1`.

### Hash verification

The SHA-256 digest is always computed during the download and printed to stderr:

```text
✔ Saved to Win11_English_x64.iso
  SHA-256: b5bb9d8014a0f9b1d61e21e796d78dccdf1352f23cd32812f4850b878ae4944c
```

Pass `--verify` to assert an expected hash — the process exits with an error on mismatch:

```sh
wisodown --edition x64 --language English \
         --verify b5bb9d8014a0f9b1d61e21e796d78dccdf1352f23cd32812f4850b878ae4944c
```

### Other flags

```sh
# List available languages for an edition
wisodown --edition win10 --list-languages

# Print the download URL without downloading
wisodown --edition x64 --language English --url-only
```

### Anti-bot cookies (if downloads fail)

Microsoft's API uses a Sentinel bot-detection layer. The tool attempts to obtain the required cookies automatically, but if it fails you can supply them manually:

1. Open the download page in a browser
2. Open DevTools → Network → any request to `microsoft.com` → Request Headers → Cookie
3. Copy the cookie string and pass it:

```sh
wisodown --edition x64 --language English --cookie "MUID=...; fptctx2=..."
```

### Debug mode

```sh
wisodown --edition x64 --language English --debug
```

Prints all API requests, responses, and cookie exchange steps to stderr.

## How it works

1. Loads the Microsoft software download page to acquire session cookies
2. Calls `ov-df.microsoft.com` to simulate the browser fingerprinting step (fptctx2/MUID cookies) that Sentinel requires
3. Calls `getSkuInformationByProductEdition` to list available language SKUs
4. Calls `GetProductDownloadLinksBySku` to get a time-limited CDN link
5. Streams the ISO to disk, computing SHA-256 on the fly, with a progress bar

For the legacy **Windows 7 / 8 / 8.1** editions there is no Microsoft API call:
the tool queries the **archive.org metadata API** for the ISO list (names, sizes,
MD5) and downloads from `https://archive.org/download/<item>/<file>`, reusing the
same multi-threaded downloader and checking the archive.org MD5.

## Documentation

- [Technical reference (English)](docs/guide/technical.md)
- [Referencia técnica (Español)](docs/guide/technical.es.md)

## License

GNU General Public License 3.0
