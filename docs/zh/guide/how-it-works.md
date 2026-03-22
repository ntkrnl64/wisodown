# 工作原理

本页介绍 `wisodown` 从会话建立到文件交付的完整运行流程。

## 概述

```
wisodown
  │
  ├─ 1. 会话初始化      — 加载页面，获取 httpOnly Cookie
  ├─ 2. 动态配置        — 从 SDS JS 文件中提取 profile/instanceId/orgId
  ├─ 3. 浏览器指纹      — VLSC (ThreatMetrix) + ov-df (fptctx2 + MUID)
  ├─ 4. SKU 列表        — getSkuInformationByProductEdition
  ├─ 5. 下载链接        — GetProductDownloadLinksBySku
  ├─ 6. 页面哈希        — 从微软页面抓取 SHA-256 表格
  └─ 7. 下载            — 并行 Range 请求 → .part 文件 → 重命名
```

---

## 1. 会话初始化

`wisodown` 创建一个随机的 UUID v4 **会话 ID** — 与浏览器端 JavaScript 通过 `crypto.randomUUID()` 生成的完全相同。此 ID 将同一会话中的所有 API 请求关联在一起。

然后对版本对应的下载页面（例如 `/software-download/windows11`）执行 `GET` 请求，使用 Mac/Chrome 的 `User-Agent` 字符串。这样做是因为：

- 使用 Windows User-Agent 时，页面会重定向到媒体创建工具，而不是显示 ISO 下载表单。
- `GET` 请求会使微软服务器通过 `Set-Cookie` 响应头设置多个 `httpOnly` 会话 Cookie。这些 Cookie 存储在 reqwest 的 Cookie 容器中，并在后续所有发往 `microsoft.com` 的请求中自动携带。

---

## 2. 动态配置

下载页面嵌入了一个 JavaScript 文件（`sds/components/content/sdsbase/…/site.ACSHASH*.min.js`），其中包含：

- **`profile`** — API 配置令牌（例如 `606624d44113`）
- **`instanceId`** — ov-df 指纹实例 UUID
- **`orgId`** — ThreatMetrix 组织 ID（例如 `y6jn8c31`）

`wisodown` 获取页面 HTML，找到 `<script src="…sdsbase…">` 标签，获取 JS 文件，并动态提取这三个值。它还会读取 `endpoint-svc` 隐藏 `<input>` 以获取 API 基础地址。这意味着微软发布的新值会被自动获取，无需更新工具。

---

## 3. 浏览器指纹握手

微软的下载 API 受 **Sentinel** 反机器人检测层保护。浏览器页面运行着**两个**独立的指纹系统：

### VLSC / ThreatMetrix

1. 加载 `https://vlscppe.microsoft.com/fp/tags.js?org_id=<orgId>&session_id=<uuid>` — 一个分析脚本，用于设置跟踪 Cookie。
2. 访问 iframe 端点 `https://vlscppe.microsoft.com/tags?org_id=<orgId>&session_id=<uuid>`，设置额外的 Cookie。

### ov-df（设备指纹）

1. 加载 `https://ov-df.microsoft.com/mdt.js?instanceId=<id>&pageId=si&session_id=<uuid>` 以获取每个会话的 `ticks` 令牌。
2. 加载由该令牌构建的指纹 iframe 地址，服务器通过 `Set-Cookie` 响应头返回：
   - `fptctx2` — Sentinel 的会话验证令牌
   - `MUID` — 微软用户 ID Cookie，作用域为 `.microsoft.com`

如果缺少**任一**系统的 Cookie，API 将返回 Sentinel 拒绝响应。`wisodown` 按顺序模拟这两个流程。

---

## 4. SKU 列表 — `getSkuInformationByProductEdition`

```
GET <api_base>/getskuinformationbyproductedition
    ?profile=<profile>
    &ProductEditionId=3321        ← 版本（3321=Win11 x64, 3324=ARM64, 2618=Win10,
                                            3322=Win11 家庭中文版, 3323=Win11 专业中文版）
    &Locale=en-US
    &sessionID=<uuid>
```

返回一个 **SKU** 的 JSON 数组 — 每种语言对应一个 SKU。每个 SKU 包含：
- `Id` — 用于下一个 API 调用
- `Language` / `LocalizedLanguage` — 显示名称
- `FriendlyFileNames` — 微软将提供的 ISO 文件名

`Referer` 头必须与下载页面地址匹配，且 `fptctx2`/`MUID` Cookie 必须存在，否则 Sentinel 会阻止请求。

---

## 5. 下载链接 — `GetProductDownloadLinksBySku`

在此调用之前，`wisodown` 会重新获取下载页面以刷新短期有效的 **`CAS_PROGRAM`** Cookie（约 8 秒有效期），Sentinel 在此端点要求该 Cookie。

```
GET <api_base>/GetProductDownloadLinksBySku
    ?profile=<profile>
    &SKU=<sku_id>
    &Locale=en-US
    &sessionID=<uuid>
```

返回包含限时（约 24 小时）CDN 下载链接的**下载选项**列表。Windows 10 包含 32 位和 64 位两个选项；Windows 11 通常每种语言只有一个选项。

---

## 6. 页面哈希抓取

微软在下载页面的 HTML 表格中公布了每个 ISO 的 SHA-256 哈希值：

```html
<td>English 64-bit</td>
<td>768984706B909479417B2368438909440F2967FF05C6A9195ED2667254E465E3</td>
```

`wisodown` 再次获取页面 HTML（与会话初始化的访问分开），并通过简单遍历解析这些键值对：查找相邻的 `<td>` 对，其中第二个单元格恰好为 64 个十六进制字符，然后构建从小写的"语言+架构"键到小写哈希值的映射。

匹配逻辑：
1. 从下载选项名称中推断 `64-bit` 或 `32-bit`。
2. 尝试精确匹配键（`"english 64-bit"`）。
3. 回退到前缀匹配以处理 `"English International"` 等变体。

当传入 `--no-verify` 时，此步骤会被跳过。

---

## 7. 并行下载

### 临时文件方式

ISO 始终写入与最终目标文件同目录下的 `.part` 文件（例如 `Win11_English_x64.iso.part`）。下载完成后原子性地重命名为最终路径。这确保了：

- 未完成的下载永远不会以完整文件的形式出现在目标路径。
- Ctrl+C 或任何错误都会干净地删除 `.part` 文件。
- 在新下载验证完成之前，不会覆盖目标位置的现有文件。

### Range 请求（多线程）

当 `--threads N` 大于 1（默认值：8）时：

1. 发送 `HEAD` 请求检查 `Accept-Ranges: bytes` 和 `Content-Length`。如果缺少任一项或不支持 Range 请求，则回退到单连接模式。
2. 使用 `set_len(total)` 在磁盘上**预分配**文件空间，以避免碎片并确保所有偏移量可写。
3. 将文件范围分割为 N 个等大的段。每个段在各自的 tokio 任务中使用 HTTP `Range: bytes=start-end` 头进行请求。
4. 每个任务打开自己的 OS 文件句柄（通过 `std::fs::File::open` + `seek`），预定位到正确的偏移量，然后用 `tokio::io::BufWriter` 包装。由于每个句柄写入不重叠的字节范围，因此不需要加锁。
5. 共享的 `ProgressBar` 由每个任务在接收数据块时递增更新。
6. 取消操作使用 `tokio::select!` — 主任务将 `join_all(tasks)` future 与 `tokio::signal::ctrl_c()` 进行竞争。按下 Ctrl+C 时，删除 `.part` 文件并以退出码 130 退出进程。

### SHA-256 计算

- **单线程模式**：SHA-256 在流式传输循环中实时计算 — 零额外 I/O 开销。
- **多线程模式**：由于数据块以任意偏移量到达，流内哈希计算需要重新排序。因此在所有任务完成后，会按顺序读取完成的 `.part` 文件，以 8 MB 为单位计算摘要。
- **`--no-verify`**：多线程模式下的重新读取步骤会被完全跳过。

---

## 错误处理

| 情况 | 行为 |
| ---- | ---- |
| 下载过程中按 Ctrl+C | 删除 `.part` 文件，退出码 130 |
| 下载流错误 | 删除 `.part` 文件，报告错误 |
| 哈希不匹配 | 文件**保留**在目标位置，报告错误（用户可自行检查） |
| 服务器不支持 Range | 透明回退到单连接模式 |
| API Sentinel 拦截 | 报告错误并提示使用 `--cookie` |
