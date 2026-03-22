<#
.SYNOPSIS
    Download a Windows ISO link using the wisodown API.

.DESCRIPTION
    Interactive one-click script that uses the wisodown API (wiso.krnl64.win)
    to generate a direct download link for a Windows ISO from Microsoft.

.PARAMETER ApiBase
    Base URL of the wisodown API. Defaults to https://wiso.krnl64.win

.PARAMETER Edition
    Edition alias: x64, arm64, win10, win11-cn-home, win11-cn-pro.
    If omitted, you'll be prompted to choose.

.PARAMETER Language
    Language name (e.g. English, French). If omitted, you'll be prompted.

.PARAMETER Download
    If set, downloads the ISO directly instead of just printing the link.

.EXAMPLE
    .\get-iso.ps1
    .\get-iso.ps1 -Edition x64 -Language English
    .\get-iso.ps1 -Edition arm64 -Language Japanese -Download
#>
param(
    [string]$ApiBase = "https://wiso.krnl64.win",
    [string]$Edition,
    [string]$Language,
    [switch]$Download
)

$ErrorActionPreference = "Stop"

function Write-Step($msg) { Write-Host "-> $msg" -ForegroundColor Cyan }
function Write-Ok($msg) { Write-Host "[OK] $msg" -ForegroundColor Green }
function Write-Err($msg) { Write-Host "[ERROR] $msg" -ForegroundColor Red }

# ── 1. Select edition ────────────────────────────────────────────────────────

$editions = @(
    @{ key = "x64";           label = "Windows 11 (x64)" }
    @{ key = "arm64";         label = "Windows 11 (ARM64)" }
    @{ key = "win10";         label = "Windows 10" }
    @{ key = "win11-cn-home"; label = "Windows 11 Home China" }
    @{ key = "win11-cn-pro";  label = "Windows 11 Pro China" }
)

if (-not $Edition) {
    Write-Host "`nWindows ISO Downloader" -ForegroundColor White
    Write-Host "======================" -ForegroundColor DarkGray
    Write-Host ""
    for ($i = 0; $i -lt $editions.Count; $i++) {
        Write-Host "  [$($i + 1)] $($editions[$i].label)"
    }
    Write-Host ""
    do {
        $choice = Read-Host "Select edition (1-$($editions.Count))"
    } while ($choice -lt 1 -or $choice -gt $editions.Count)
    $Edition = $editions[$choice - 1].key
}

# ── 2. Fetch languages ───────────────────────────────────────────────────────

Write-Step "Fetching languages for '$Edition'..."
$skus = Invoke-RestMethod -Uri "$ApiBase/api/skus?edition=$Edition"
if ($skus.error) {
    Write-Err $skus.error
    exit 1
}

if (-not $Language) {
    Write-Host ""
    for ($i = 0; $i -lt $skus.Count; $i++) {
        Write-Host "  [$($i + 1)] $($skus[$i].LocalizedLanguage) ($($skus[$i].Language))"
    }
    Write-Host ""
    do {
        $choice = Read-Host "Select language (1-$($skus.Count))"
    } while ($choice -lt 1 -or $choice -gt $skus.Count)
    $Language = $skus[$choice - 1].Language
}

# ── 3. Get download links ────────────────────────────────────────────────────

Write-Step "Generating download link..."
$links = Invoke-RestMethod -Uri "$ApiBase/api/links?edition=$Edition&language=$Language"
if ($links.error) {
    Write-Err $links.error
    exit 1
}

Write-Host ""
Write-Ok "$($links.localizedLanguage)"
if ($links.filename) {
    Write-Host "  File: $($links.filename)" -ForegroundColor DarkGray
}
if ($links.expiresAt) {
    Write-Host "  Expires: $($links.expiresAt)" -ForegroundColor DarkGray
}

Write-Host ""
foreach ($dl in $links.downloads) {
    Write-Host "  $($dl.name)" -ForegroundColor White
    Write-Host "  $($dl.url)" -ForegroundColor Yellow
    Write-Host ""
}

# ── 4. Show hashes ───────────────────────────────────────────────────────────

if ($links.hashes -and $links.hashes.PSObject.Properties.Count -gt 0) {
    Write-Host "SHA-256 Hashes:" -ForegroundColor White
    foreach ($prop in $links.hashes.PSObject.Properties) {
        Write-Host "  $($prop.Name): $($prop.Value)" -ForegroundColor DarkGray
    }
    Write-Host ""
}

# ── 5. Download if requested ─────────────────────────────────────────────────

if ($Download -and $links.downloads.Count -gt 0) {
    $url = $links.downloads[0].url
    $filename = if ($links.filename) { $links.filename } else { "windows.iso" }
    Write-Step "Downloading $filename..."
    Invoke-WebRequest -Uri $url -OutFile $filename -UseBasicParsing
    Write-Ok "Saved to $filename"
}
