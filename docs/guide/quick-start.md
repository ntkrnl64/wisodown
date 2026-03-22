# Quick Start

## Use online

No installation needed — visit **[wiso.krnl64.win](https://wiso.krnl64.win)** to download ISOs directly from your browser. Select an edition, pick a language, and get a direct download link from Microsoft.

## Install locally

You need [Rust](https://rustup.rs) installed.

```sh
git clone https://github.com/ntkrnl64/win11_iso
cd win11_iso
cargo install --path .
```

The binary is called `wisodown`.

## Download your first ISO

Run without arguments to launch the interactive wizard:

```sh
wisodown
```

You'll be prompted to pick:
1. **Windows version** — Windows 11 x64, Windows 11 arm64, Windows 11 Home/Pro (China), or Windows 10
2. **Language** — any language Microsoft distributes (defaults to English)
3. **Architecture** (Windows 10 only) — 64-bit or 32-bit

The ISO downloads to your current directory. SHA-256 is fetched from Microsoft's page automatically and verified against the download.

## Non-interactive download

Skip all prompts by passing flags:

```sh
# Windows 11 x64, English
wisodown --edition x64 --language English

# Windows 10, French, save to ~/Downloads
wisodown --edition win10 --language French --output ~/Downloads
```

## What you'll see

```
➜ Initializing session for Windows 11 (x64)…
➜ Fetching available languages…
➜ Selected: English (SKU 5585)
➜ Requesting download link…
  ℹ Link expires: 2026-03-22T18:00:00Z
➜ Fetching integrity hashes from Microsoft…
  ℹ Expected SHA-256: 768984706b909479...
⬇ Downloading Windows 11 English 64-bit → Win11_24H2_English_x64.iso
  ⣾ [████████████████░░░░░░░░] 2.4 GB/6.0 GB (4m32s) 22 MB/s

✔ Saved to Win11_24H2_English_x64.iso
  SHA-256: 768984706b909479...
✔ Hash verified
```

Press **Ctrl+C** at any time to cancel — the partial file is removed automatically.
