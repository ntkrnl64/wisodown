---
layout: home

hero:
  name: Windows ISO 下载器
  text: 直接来自微软，无需浏览器。
  tagline: 直接从微软 CDN 下载 Windows 10 和 Windows 11 ISO 镜像，自动进行 SHA-256 校验。
  actions:
    - theme: brand
      text: 在线下载
      link: https://wiso.krnl64.win
    - theme: alt
      text: 快速开始
      link: /zh/guide/quick-start
    - theme: alt
      text: 使用参考
      link: /zh/guide/usage

features:
  - title: 官方来源
    details: 使用与微软下载页面相同的 API，直接从微软 CDN 下载 ISO 镜像。不经过任何第三方镜像。
  - title: SHA-256 校验
    details: 自动从微软页面获取哈希值，并与下载文件进行比对验证。无需手动查找哈希值。
  - title: Windows 10 和 11
    details: 支持 Windows 11 x64、Windows 11 ARM64 以及 Windows 10 多版本（家庭版/专业版，32 位和 64 位）。也支持自定义版本 ID。
  - title: 交互式或脚本化
    details: 不带参数运行可启动交互式向导，或通过命令行参数在脚本和 CI 中实现全自动下载。
---

::: warning IP 封禁
请注意，如果向微软发送过多请求，您的 IP 可能会被临时封禁。
届时您会收到 `[ErrorSettings.SentinelReject] Sentinel marked this request as rejected. (type 9)` 错误。
请使用 VPN 来发送更多请求（笑），或者稍后再试。
如果仍然无法解决，请在 Issues 中反馈并提供使用 `--debug` 参数生成的日志。
:::
