# 使用参考

## 命令格式

```
wisodown [OPTIONS]
```

不带选项运行即可启动交互式向导。

## 选项

| 参数 | 说明 |
| ---- | ---- |
| `-e, --edition <ALIAS>` | 版本别名：`x64`、`arm64`、`win10`、`win11-cn-home`、`win11-cn-pro`，以及 archive.org 版本 `win7`、`win8`、`win8.1` |
| `--edition-id <N>` | 原始数字 `ProductEditionId`（高级用法） |
| `--page-url <URL>` | 覆盖用于获取 Cookie 的页面地址（需配合 `--edition-id` 使用） |
| `-l, --language <LANG>` | 语言名称，例如 `English`、`French`、`Japanese`。不区分大小写。旧版版本优先精确匹配，否则按文件名子串匹配。 |
| `-o, --output <DIR>` | 输出目录（默认：当前目录） |
| `-t, --threads <N>` | 下载并行连接数（默认：`8`） |
| `--url-only` | 仅输出下载链接，不执行下载 |
| `--list-languages` | 列出所选版本可用的语言并退出（旧版版本则列出 ISO 文件） |
| `--cookie <STR>` | 浏览器 Cookie 字符串，用于手动绕过反机器人检测 |
| `--verify <SHA256>` | 指定期望的 SHA-256 十六进制摘要（覆盖页面哈希值）。与 `--no-verify` 互斥。 |
| `--no-verify` | 完全跳过 SHA-256 获取、计算和校验。在多线程模式下也会跳过重新读取哈希的步骤。 |
| `--debug` | 将原始 API 请求和响应输出到 stderr |

## 版本别名

| `--edition` | Windows 版本 | 架构 | ISO 文件名 |
| ----------- | ------------ | ---- | ---------- |
| `x64` | Windows 11 **25H2** | x64 (64 位) | `Win11_25H2_<Lang>_x64_v2.iso` |
| `arm64` | Windows 11 **25H2** | Arm64 | `Win11_25H2_<Lang>_Arm64_v2.iso` |
| `win10` | Windows 10 **22H2** | x86 + x64（提示时选择） | `Win10_22H2_<Lang>_x32v1.iso` / `…_x64v1.iso` |
| `win11-cn-home` | Windows 11 **25H2** 家庭中文版 | x64 | 仅简体中文 |
| `win11-cn-pro` | Windows 11 **25H2** 专业中文版 | x64 | 仅简体中文 |
| `win7` | Windows 7 **Ultimate SP1** *（archive.org）* | x86 + x64 | `Win7_Ult_SP1_English_x32.iso` / `Win7_Ult_SP1_Spanish_x32.iso` / `…_x64.iso` / `…_x64.es.iso` |
| `win8` | Windows 8 **RTM** *（archive.org）* | x86 + x64 | `en_windows_8_x86_dvd_915417.iso` / `…_x64_…iso` |
| `win8.1` | Windows 8.1 **家庭版/专业版** *（archive.org）* | x86 + x64 | `Win8.1_<Lang>_x32.iso` / `Win8.1_<Lang>_x64.iso` |

微软只提供每个产品的最新功能更新（Windows 11 **25H2**、Windows 10 **22H2**）。
消费者 ISO 为**多版本**，Windows 10 和 Windows 11 x64/Arm64 均提供 38 种语言。

`win7`、`win8`、`win8.1` 从 **archive.org** 下载（微软已下架这些消费者 ISO）。
`win8.1` 支持多语言，`win7`/`win8` 支持 **中英文**（西班牙语/English ISO 通过 archive.org 可用）。下载会对照 archive.org 公布的
**MD5** 进行校验；`--verify` 仍可传入 SHA-256。

也接受以下别名：`win11`、`win11-x64`、`win11-arm64`、`amd64`、`aarch64`、`cn-home`、`cn-pro`。旧版版本还接受 `windows7`、`windows8`、`windows8.1`、`win81`、`windows81` 以及裸值 `7`、`8`、`8.1`。

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

## Ciclo de vida del soporte de Windows

La política de soporte de Microsoft afecta a qué versiones de ISO puedes usar con seguridad con esta herramienta:

- **Windows 7** el soporte principal terminó el **13 de enero de 2015**, el soporte extendido terminó el **14 de enero de 2020**. Ya no recibe actualizaciones de seguridad.
- **Windows 8** el soporte terminó el **12 de enero de 2016**. Ya no recibe actualizaciones de seguridad.
- **Windows 8.1** el soporte terminó el **10 de enero de 2018**. Ya no recibe actualizaciones de seguridad.
- **Windows 10** el soporte principal terminó el **13 de octubre de 2020**. El soporte extendido termina el **14 de octubre de 2025**.
- **Windows 10 ESU** (Actualizaciones de seguridad extendidas) está disponible para Windows 10 Enterprise hasta **enero de 2028** (a través de Licencias por Volumen). ESU recibe actualizaciones de seguridad críticas incluso después de que termina el soporte extendido.
- **Rotación de certificados raíz**: A partir de 2023, Microsoft comenzó a rotar los certificados raíz de confianza. Los sistemas que ejecutan Windows 7/8/8.1 **sin ESU** pueden perder la capacidad de conectarse a Windows Update y algunos servicios de Microsoft a medida que expiran las raíces antiguas y no se instalan las nuevas. Windows 10 con ESU recibe las actualizaciones de certificados. Esto puede afectar la capacidad de la herramienta para adquirir cookies de la API de Microsoft en sistemas sin actualizar.

> **Recomendación**: Si aún se requiere para flujos de trabajo heredados, use Windows 10 con ESU, o migre a un SO actualmente compatible. La herramienta misma se ejecuta en cualquier SO con Rust, pero los ISOs de Windows que descarga tienen el ciclo de vida anterior.

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

## 旧版版本（Windows 7 / 8 / 8.1）

这些版本从 **archive.org** 获取，而非微软：

```sh
# Windows 7 Ultimate SP1，64 位（中文/Chinese）
wisodown --edition win7 --language Chinese(Simplified)

# Windows 7 Ultimate SP1，64 位（英文/English）
wisodown --edition win7 --language English

# Windows 8 RTM，32 位（英文/English）
wisodown --edition win8 --language English

# Windows 8 RTM，64 位（英文/English）
wisodown --edition win8 --language English

# Windows 8.1（多语言，提供 32 位与 64 位）
wisodown --edition win8.1 --language Spanish

# 列出该版本的全部 ISO
wisodown --edition win8.1 --list-languages

# 仅输出 archive.org 链接
wisodown --edition win7 --language English --url-only
```

ISO 列表（名称、大小、MD5）在运行时从 archive.org 元数据 API 读取，因此始终
保持最新。由于许多语言同时提供 32 位和 64 位文件，语言查询可能匹配多个 ISO，
此时会出现选择菜单；下载完成后会对照 archive.org 的 MD5 进行校验。`win7` y
`win8` admiten inglés y español; `win8.1` usa coincidencia por subcadena en el
nombre de archivo.

## 列出可用语言

```sh
wisodown --edition win10 --list-languages
wisodown --edition x64 --list-languages
```

输出示例：

```
Available languages:
  Arabic                         (id: 20035, file: Win11_25H2_Arabic_x64_v2.iso)
  English                        (id: 20046, file: Win11_25H2_English_x64_v2.iso)
  French                         (id: 20047, file: Win11_25H2_French_x64_v2.iso)
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
