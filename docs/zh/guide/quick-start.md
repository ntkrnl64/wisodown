# 快速开始

## 下载预编译版本

无需安装 — 从 [GitHub Releases](https://github.com/BENDER35/wisodown-lin/releases/latest) 下载适用于 Linux、macOS 或 Windows（64 位 / 32 位）的压缩包。

## 自建网页版

运行自带的服务器后，在浏览器中打开 <http://localhost:3000>：

```sh
cargo run --release --features server -- --port 3000
```

## 本地安装

需要先安装 [Rust](https://rustup.rs)。

```sh
git clone https://github.com/BENDER35/wisodown-lin
cd wisodown-lin
cargo install --path .
```

可执行文件名为 `wisodown`。

## 下载你的第一个 ISO

不带参数运行即可启动交互式向导：

```sh
wisodown
```

系统会提示你选择：
1. **Windows 版本** — Windows 11 x64、Windows 11 arm64、Windows 11 家庭版/专业版（中国版）、Windows 10，或来自 archive.org 的 Windows 7 / 8 / 8.1
2. **语言** — 微软提供的所有语言（默认为英语）；Windows 8.1 则从 archive.org 的语言中选择
3. **架构**（Windows 10 及旧版版本）— 64 位或 32 位

ISO 文件将下载到当前目录。SHA-256 哈希值会自动从微软页面获取并与下载文件校验；Windows 7/8/8.1 则对照 archive.org 的 MD5 校验。Windows 7/8 支持中英文。

## 非交互式下载

通过传递参数可跳过所有提示：

```sh
# Windows 11 x64，英语
wisodown --edition x64 --language English

# Windows 10，法语，保存到 ~/Downloads
wisodown --edition win10 --language French --output ~/Downloads

# Windows 7 Ultimate SP1（archive.org），英语
wisodown --edition win7 --language English

# Windows 7 Ultimate SP1（archive.org），西班牙语/Spanish
wisodown --edition win7 --language Spanish
```

## 运行效果

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

随时按 **Ctrl+C** 即可取消 — 未完成的文件会自动删除。
