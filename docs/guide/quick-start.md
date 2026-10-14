# Quick Start

## Download prebuilt binaries

Grab an archive for **Linux, macOS or Windows (64-bit / 32-bit)** from
**[GitHub Releases](https://github.com/BENDER35/wisodown-lin/releases/latest)**.

## Run the web UI locally

The binary ships an optional server with an embedded web UI. Start it and open
<http://localhost:3000>:

```sh
cargo run --release --features server -- --port 3000
```

## Install locally

You need [Rust](https://rustup.rs) installed.

```sh
git clone https://github.com/BENDER35/wisodown-lin
cd wisodown-lin
cargo install --path .
```

The binary is called `wisodown`.

## Download your first ISO

Run without arguments to launch the interactive wizard:

```sh
wisodown
```

You'll be prompted to pick:
1. **Windows version** — Windows 11 x64, Windows 11 arm64, Windows 11 Home/Pro (China), Windows 10, or the archive.org editions Windows 7 / 8 / 8.1
2. **Language** — any language Microsoft distributes (defaults to English); for Windows 8.1 you pick from the archive.org languages
3. **Architecture** (Windows 10 and the legacy editions) — 64-bit or 32-bit

The ISO downloads to your current directory. SHA-256 is fetched from Microsoft's page automatically and verified against the download; Windows 7/8/8.1 downloads are verified against archive.org's MD5 instead. Windows 7/8 support English and Spanish.

## Non-interactive download

Skip all prompts by passing flags:

```sh
# Windows 11 x64, English
wisodown --edition x64 --language English

# Windows 10, French, save to ~/Downloads
wisodown --edition win10 --language French --output ~/Downloads

# Windows 7 Ultimate SP1 (archive.org), English
wisodown --edition win7 --language English

# Windows 7 Ultimate SP1 (archive.org), Spanish
wisodown --edition win7 --language Spanish
```

## What you'll see

```
➜ Initializing session for Windows 11 (x64)…
➜ Fetching available languages…
➜ Selected: English (SKU 20046)
➜ Requesting download link…
  ℹ Link expires: 2026-10-15T18:00:00Z
➜ Fetching integrity hashes from Microsoft…
  ℹ Expected SHA-256: 768984706b909479...
⬇ Downloading Windows 11 English 64-bit → Win11_25H2_English_x64_v2.iso
  ⣾ [████████████████░░░░░░░░] 2.4 GB/7.2 GB (4m32s) 22 MB/s

✔ Saved to Win11_25H2_English_x64_v2.iso
  SHA-256: 768984706b909479...
✔ Hash verified
```

Press **Ctrl+C** at any time to cancel — the partial file is removed automatically.
