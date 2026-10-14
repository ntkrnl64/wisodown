<#
.SYNOPSIS
    Download a Windows ISO link using the wisodown API.

.DESCRIPTION
    Interactive script that uses the wisodown API (defaults to a local server
    on http://localhost:3000) to generate a direct download link for a Windows
    ISO from Microsoft — plus Windows 7/8/8.1 from archive.org.

    Runs on Windows PowerShell 5.1+ and on PowerShell 7+ (pwsh) on Linux and
    macOS. On Linux/macOS, make it executable and run it directly, or invoke
    it through pwsh:

        chmod +x ./get-iso.ps1
        ./get-iso.ps1 -Edition x64 -Language English
        # or, with legacy editions:
        ./get-iso.ps1 -e win7
        ./get-iso.ps1 -e win8.1 -l Spanish -Download

.PARAMETER ApiBase
    Base URL of the wisodown API. Defaults to $env:WISODOWN_API, or
    http://localhost:3000 when that is not set.

.PARAMETER Edition
    Edition alias.

    Microsoft editions: x64, arm64, win10, win11-cn-home, win11-cn-pro.

    Archive.org editions (Windows 7/8/8.1): win7, win8, win8.1.

.PARAMETER Language
    Language name (e.g. English, French, Spanish).
    For legacy editions (win7/win8) only English is available.
    For win8.1 it is a case-insensitive substring match on the ISO filename.

.PARAMETER Download
    If set, downloads the ISO directly instead of just printing the link.

.PARAMETER ListLanguages
    If set, lists the available ISO files for the chosen edition and exits.

.PARAMETER UrlOnly
    If set, prints the download URL without downloading the file.

.PARAMETER Cookie
    Browser cookie string for manual anti-bot bypass (Sentinel bypass).

.EXAMPLE
    .\get-iso.ps1
    .\get-iso.ps1 -Edition x64 -Language English
    .\get-iso.ps1 -Edition arm64 -Language Japanese -Download
    .\get-iso.ps1 -Edition win7
    .\get-iso.ps1 -Edition win8.1 -Language Spanish -ListLanguages
    .\get-iso.ps1 -Edition win7 -UrlOnly
#>
[CmdletBinding()]
param(
    [string]$ApiBase = $(if ($env:WISODOWN_API) { $env:WISODOWN_API } else { "http://localhost:3000" }),
    [string]$Edition,
    [string]$Language,
    [switch]$Download,
    [switch]$ListLanguages,
    [switch]$UrlOnly,
    [string]$Cookie
)

function Write-Step($msg) { Write-Host "-> $msg" -ForegroundColor Cyan }
function Write-Ok($msg)  { Write-Host "[OK]  $msg" -ForegroundColor Green }
function Write-Err($msg) { Write-Host "[ERR] $msg" -ForegroundColor Red }
function Write-Dim($msg) { Write-Host "    $msg" -ForegroundColor DarkGray }

# ── Helper: read a valid choice number ───────────────────────────────────────
function Read-Choice {
    param([int]$Min, [int]$Max)
    while ($true) {
        $raw = Read-Host "Enter a number between $Min and $Max"
        if ($raw -as [int] -and $raw -ge $Min -and $raw -le $Max) {
            return $raw
        }
        Write-Err "Invalid input – enter a number between $Min and $Max."
    }
}

# ── Connectivity check (archive.org) ────────────────────────────────────────
function Check-ArchiveOrg {
    try {
        $result = Invoke-WebRequest -Uri "https://archive.org" -Method HEAD -TimeoutSec 5
        return $true
    } catch {
        Write-Warn "⚠ archive.org seems unreachable – continuing anyway."
        return $false
    }
}

# ── Edition catalogue (Microsoft + archive.org) ─────────────────────────────
$editionCatalog = @(
    @{ key = "x64";           label = "Windows 11 (x64)"                ; legacy = $false },
    @{ key = "arm64";         label = "Windows 11 (ARM64)"            ; legacy = $false },
    @{ key = "win10";         label = "Windows 10 (22H2)"             ; legacy = $false },
    @{ key = "win11-cn-home"; label = "Windows 11 Home China"       ; legacy = $false },
    @{ key = "win11-cn-pro";  label = "Windows 11 Pro China"          ; legacy = $false },
    @{ key = "win7";          label = "Windows 7 (archive.org)"       ; legacy = $true  },
    @{ key = "win8";          label = "Windows 8 (archive.org)"       ; legacy = $true  },
    @{ key = "win8.1";         label = "Windows 8.1 (archive.org)"     ; legacy = $true  }
)

# ── Parse parameters ────────────────────────────────────────────────────────
if (-not $Edition) {
    # Interactive selection
    Write-Host "`nWindows ISO Downloader" -ForegroundColor White
    Write-Host "======================" -ForegroundColor DarkGray
    Write-Host ""
    for ($i = 0; $i -lt $editionCatalog.Count; $i++) {
        $lbl = $editionCatalog[$i].label
        if ($editionCatalog[$i].legacy) {
            Write-Host "  [$($i + 1)] $lbl (archive.org)"
        } else {
            Write-Host "  [$($i + 1)] $lbl"
        }
    }
    Write-Host ""
    $max = $editionCatalog.Count
    $choice = Read-Choice 1 $max
    $Edition = $editionCatalog[$choice - 1].key
}

# Normalise edition to lower case for consistency
$Edition = $Edition.ToLower()

# Validate edition
$editionValid = $false
foreach ($ed in $editionCatalog) {
    if ($ed.key -eq $Edition) {
        $editionValid = $true
        break
    }
}
if (-not $editionValid) {
    Write-Err "Unknown edition: $Edition."
    Write-Host "Valid editions:"
    foreach ($ed in $editionCatalog) {
        Write-Host "  $($ed.key) – $($ed.label)"
    }
    exit 1
}

# ── Archive.org connectivity check ──────────────────────────────────────────
if (($editionCatalog | Where { $editionCatalog.key -eq $Edition }).legacy -and \
    (Check-ArchiveOrg -eq $false)) {
    Write-Warn "archive.org unreachable – some features may be limited."
}

# ── Determine if this is a legacy edition ───────────────────────────────────
$isLegacy = ($editionCatalog | Where { $editionCatalog.key -eq $Edition }).legacy

# ── Fetch languages ──────────────────────────────────────────────────────────
Write-Step "Fetching languages for '$Edition'..."

# For legacy editions we know the language situation up-front:
# win7/win8 → English only; win8.1 → multilingual (API returns all SKUs)
if ($isLegacy -and $Edition -ne "win8.1") {
    # win7 or win8: only English ISOs exist
    $Language = "English"
    Write-Step "This edition only ships English ISOs – language set to English."
} else {
    # Microsoft edition or win8.1: query the API
    try {
        $skus = @(Invoke-RestMethod -Uri "$ApiBase/api/skus?edition=$([uri]::EscapeDataString($Edition))")
    } catch {
        Write-Err "Request failed: $($_.Exception.Message)"
        Write-Host "        Start a local server:" -ForegroundColor Red
        Write-Host "          cargo run --release --features server -- --port 3000" -ForegroundColor Red
        Write-Host "        or point elsewhere with -ApiBase <url> / WISODOWN_API=<url>." -ForegroundColor Red
        exit 1
    }
    if ($skus.Count -eq 1 and $skus[0].PSObject.Properties['error']) {
        Write-Err $skus[0].error
        exit 1
    }

    if (-not $Language) {
        Write-Host ""
        for ($i = 0; $i -lt $skus.Count; $i++) {
            Write-Host "  [$($i + 1)] $($skus[$i].LocalizedLanguage) ($($skus[$i].Language))"
        }
        Write-Host ""
        $choice = Read-Choice 1 $skus.Count
        $Language = $skus[$choice - 1].Language
    }
}

# ── Get download links ───────────────────────────────────────────────────────
Write-Step "Generating download link..."
try {
    $langEncoded = [uri]::EscapeDataString($Language)
    $links = Invoke-RestMethod -Uri "$ApiBase/api/links?edition=$([uri]::EscapeDataString($Edition))&language=$langEncoded"
} catch {
    Write-Err "Request failed: $($_.Exception.Message)"
    Write-Host "        Start a local server:" -ForegroundColor Red
    Write-Host "          cargo run --release --features server -- --port 3000" -ForegroundColor Red
    Write-Host "        or point elsewhere with -ApiBase <url> / WISODOWN_API=<url>." -ForegroundColor Red
    exit 1
}
if ($links.PSObject.Properties['error']) {
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

# ── Handle output modes ──────────────────────────────────────────────────────
$downloads = @($links.downloads)

# --UrlOnly: print URL and exit
if ($UrlOnly) {
    if ($downloads.Count -gt 0) {
        Write-Host "$($downloads[0].url)"
    } else {
        Write-Err "No download links available."
        exit 1
    }
    exit 0
}

# --ListLanguages: list ISO files ───────────────────────────────────────────
if ($ListLanguages) {
    Write-Host ""
    Write-Host "=== Available ISOs for $Edition ===" -ForegroundColor Cyan
    if ($isLegacy) {
        switch ($Edition) {
            "win7" {
                Write-Host "  Win7_Ult_SP1_English_x32.iso   2.4 GiB" -ForegroundColor White
                Write-Host "  Win7_Ult_SP1_English_x64.iso   3.1 GiB" -ForegroundColor White
            }
            "win8" {
                Write-Host "  en_windows_8_x64_dvd_915440.iso   3.3 GiB" -ForegroundColor White
                Write-Host "  en_windows_8_x86_dvd_915417.iso   2.5 GiB" -ForegroundColor White
            }
            "win8.1" {
                Write-Host "  (Windows 8.1 has 74 ISOs – showing a sample)" -ForegroundColor DarkGray
                Write-Host "  Win8.1_Arabic_x32.iso          3.0 GiB" -ForegroundColor White
                Write-Host "  Win8.1_Arabic_x64.iso          4.0 GiB" -ForegroundColor White
                Write-Host "  Win8.1_BrazilianPortuguese_x32.iso   3.0 GiB" -ForegroundColor White
                Write-Host "  Win8.1_Bulgarian_x32.iso       2.9 GiB" -ForegroundColor White
                Write-Host "  Win8.1_Chinese(Simplified)_x32.iso   3.1 GiB" -ForegroundColor White
                Write-Host "  Win8.1_Chinese(Simplified)_x64.iso   4.2 GiB" -ForegroundColor White
            }
        }
    } else {
        # Microsoft editions: list languages from SKUs
        Write-Host "Languages available:" -ForegroundColor White
        foreach ($sku in $skus) {
            Write-Host "  $($sku.Language) (id: $($sku.id))" -ForegroundColor DarkGray
        }
    }
    Write-Host ""
    exit 0
}

# ── Download if requested ────────────────────────────────────────────────────
if ($Download -and $downloads.Count -gt 0) {
    $url = $downloads[0].url
    $filename = if ($links.filename) { $links.filename } else { "windows.iso" }
    Write-Step "Downloading $filename..."
    try {
        Invoke-WebRequest -Uri $url -OutFile $filename -Headers @{ "Cookie" = $Cookie } -UseBasicParsing
        Write-Ok "Saved to $filename"
    } catch {
        Write-Err "Download failed: $($_.Exception.Message)"
        exit 1
    }
} elseif ($Download -and $downloads.Count -eq 0) {
    Write-Err "No download links available for the selected edition/language."
    exit 1
}