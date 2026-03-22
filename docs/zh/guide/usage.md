# 使用参考

## 命令格式

```
wisodown [OPTIONS]
```

不带选项运行即可启动交互式向导。

## 选项

| 参数 | 说明 |
| ---- | ---- |
| `-e, --edition <ALIAS>` | 版本别名：`x64`、`arm64`、`win10`、`win11-cn-home`、`win11-cn-pro` |
| `--edition-id <N>` | 原始数字 `ProductEditionId`（高级用法） |
| `--page-url <URL>` | 覆盖用于获取 Cookie 的页面地址（需配合 `--edition-id` 使用） |
| `-l, --language <LANG>` | 语言名称，例如 `English`、`French`、`Japanese`。不区分大小写。 |
| `-o, --output <DIR>` | 输出目录（默认：当前目录） |
| `-t, --threads <N>` | 下载并行连接数（默认：`8`） |
| `--url-only` | 仅输出下载链接，不执行下载 |
| `--list-languages` | 列出所选版本可用的语言并退出 |
| `--cookie <STR>` | 浏览器 Cookie 字符串，用于手动绕过反机器人检测 |
| `--verify <SHA256>` | 指定期望的 SHA-256 十六进制摘要（覆盖页面哈希值）。与 `--no-verify` 互斥。 |
| `--no-verify` | 完全跳过 SHA-256 获取、计算和校验。在多线程模式下也会跳过重新读取哈希的步骤。 |
| `--debug` | 将原始 API 请求和响应输出到 stderr |

## 版本别名

| `--edition` | Windows 版本 |
| ----------- | ------------ |
| `x64` | Windows 11，64 位 (x86-64) |
| `arm64` | Windows 11，ARM 64 位 |
| `win10` | Windows 10，多版本（家庭版/专业版，32 位 + 64 位） |
| `win11-cn-home` | Windows 11 家庭中文版（仅限中国） |
| `win11-cn-pro` | Windows 11 专业中文版 |

也接受以下别名：`win11`、`win11-x64`、`win11-arm64`、`amd64`、`aarch64`、`cn-home`、`cn-pro`。

## 哈希校验

SHA-256 哈希值会在每次下载前**自动从微软下载页面获取**，并在文件流式传输过程中进行校验。你无需做任何额外操作 — 输出会显示：

```
➜ Fetching integrity hashes from Microsoft…
  ℹ Expected SHA-256: 768984706b909479…
...
  SHA-256: 768984706b909479…
✔ Hash verified
```

如果哈希不匹配，`wisodown` 会输出错误并以非零退出码退出 — **文件会保留**在目标位置，以便你手动检查或重试。

如果要使用自定义哈希值而非页面哈希值，请使用 `--verify`：

```sh
wisodown --edition x64 --language English \
  --verify 768984706b909479417b2368438909440f2967ff05c6a9195ed2667254e465e3
```

如果要完全跳过校验（在多线程模式下也会跳过重新读取步骤）：

```sh
wisodown --edition x64 --language English --no-verify
```

## 并行下载

默认情况下，`wisodown` 使用 **8 个并行连接**，通过 HTTP Range 请求进行下载，在高带宽链路上通常比单连接快 2 到 4 倍。

```sh
# 使用 8 个线程
wisodown --edition x64 --language English --threads 8

# 单连接（禁用并行模式）
wisodown --edition x64 --language English --threads 1
```

如果服务器不支持 Range 请求，`wisodown` 会自动回退到单连接模式，无论 `--threads` 设置为何值。

下载过程中文件会写入同目录下的 `.part` 文件，下载完成后才会重命名为最终文件名。这意味着：

- 中断的下载不会在目标路径留下不完整的文件。
- Ctrl+C 会立即删除 `.part` 文件。

## 使用原始版本 ID

微软网站上的每个 ISO 产品都有一个数字 `ProductEditionId`。你可以直接传入该 ID 来下载没有命名别名的版本：

```sh
wisodown --edition-id 2618 --language English
```

如果该产品位于不同的下载页面（需要正确的 Cookie），还需要提供页面地址：

```sh
wisodown --edition-id 2618 \
  --page-url https://www.microsoft.com/en-us/software-download/windows10ISO \
  --language English
```

## 列出可用语言

```sh
wisodown --edition win10 --list-languages
wisodown --edition x64 --list-languages
```

输出示例：

```
Available languages:
  Arabic                         (id: 1234, file: Win11_24H2_Arabic_x64.iso)
  English                        (id: 5585, file: Win11_24H2_English_x64.iso)
  French                         (id: 5586, file: Win11_24H2_French_x64.iso)
  ...
```

## 仅获取链接

输出有时效限制的 CDN 下载链接，不下载文件：

```sh
wisodown --edition x64 --language English --url-only
```

该链接有效期约为 24 小时。

## 反机器人 Cookie

微软的 API 使用 Sentinel 反机器人检测层。`wisodown` 会尝试通过模拟浏览器指纹握手来自动绕过。如果请求因 API 错误失败，你可以提供真实的浏览器 Cookie 作为后备方案：

1. 在浏览器中打开下载页面
2. 打开开发者工具 → 网络 → 任意发往 `microsoft.com` 的请求 → 请求头 → Cookie
3. 复制完整的 Cookie 字符串：

```sh
wisodown --edition x64 --language English \
  --cookie "MUID=abc123; fptctx2=xyz..."
```

## Ctrl+C

随时按 **Ctrl+C** 即可取消。未完成的下载文件会被删除，进程以退出码 `130` 正常退出。

## 调试模式

```sh
wisodown --edition x64 --language English --debug
```

将会话 ID、所有 HTTP 请求/响应、指纹 Cookie 步骤和哈希查找结果输出到 stderr。
