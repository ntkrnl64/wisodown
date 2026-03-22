# 快速开始

## 在线使用

无需安装 — 访问 **[wiso.krnl64.win](https://wiso.krnl64.win)** 即可直接在浏览器中下载 ISO 镜像。选择版本、选择语言，即可获取来自微软的直接下载链接。

## 本地安装

需要先安装 [Rust](https://rustup.rs)。

```sh
git clone https://github.com/ntkrnl64/win11_iso
cd win11_iso
cargo install --path .
```

可执行文件名为 `wisodown`。

## 下载你的第一个 ISO

不带参数运行即可启动交互式向导：

```sh
wisodown
```

系统会提示你选择：
1. **Windows 版本** — Windows 11 x64、Windows 11 arm64、Windows 11 家庭版/专业版（中国版）或 Windows 10
2. **语言** — 微软提供的所有语言（默认为英语）
3. **架构**（仅 Windows 10）— 64 位或 32 位

ISO 文件将下载到当前目录。SHA-256 哈希值会自动从微软页面获取，并与下载文件进行校验。

## 非交互式下载

通过传递参数可跳过所有提示：

```sh
# Windows 11 x64，英语
wisodown --edition x64 --language English

# Windows 10，法语，保存到 ~/Downloads
wisodown --edition win10 --language French --output ~/Downloads
```

## 运行效果

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

随时按 **Ctrl+C** 即可取消 — 未完成的文件会自动删除。
