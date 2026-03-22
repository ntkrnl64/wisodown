---
layout: home

hero:
  name: Windows ISO Downloader
  text: Direct from Microsoft. No browser needed.
  tagline: Download Windows 10 and Windows 11 ISOs straight from Microsoft's CDN, with automatic SHA-256 verification.
  actions:
    - theme: brand
      text: Quick Start
      link: /guide/quick-start
    - theme: alt
      text: Usage Reference
      link: /guide/usage

features:
  - title: Official Source
    details: Downloads ISOs directly from Microsoft's CDN using the same API the download pages use. No third-party mirrors.
  - title: SHA-256 Verification
    details: Hashes are fetched automatically from Microsoft's page and verified against your download. No manual hash lookup needed.
  - title: Windows 10 & 11
    details: Supports Windows 11 x64, Windows 11 ARM64, and Windows 10 multi-edition (Home/Pro, 32-bit and 64-bit). Custom edition IDs also supported.
  - title: Interactive or Scripted
    details: Run without arguments for a guided wizard, or pass flags to fully automate downloads in scripts and CI.
---

::: warning IP Ban
Please note that your IP can gets banned by Microsoft temporarily if you send too many requests.
You will receive `[ErrorSettings.SentinelReject] Sentinel marked this request as rejected. (type 9)`.
Please use a VPN to send even more requests (LOL) or give up and try again later.
If that doesn't help, please report in issues and provide logs using `--debug` flag.
:::
