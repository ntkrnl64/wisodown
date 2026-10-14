#!/usr/bin/env bash
#
# get-iso.sh — Download a Windows ISO link using the wisodown API.
#
# Usage:
#   ./get-iso.sh                              # interactive wizard
#   ./get-iso.sh -e x64 -l English            # non-interactive download
#   ./get-iso.sh -e win7 --url-only           # print archive.org URL only
#   ./get-iso.sh --edition win8.1 --list-languages  # list ISOs for win8.1
#
# Options:
#   -a URL     API base URL (default: $WISODOWN_API or http://localhost:3000)
#   -e EDITION Edition alias.
#              Microsoft: x64, arm64, win10, win11-cn-home, win11-cn-pro
#              Archive.org: win7, win8, win8.1
#   -l LANG    Language name (e.g. English, French, Spanish).
#              For legacy editions (win7/win8) matches English only;
#              for win8.1 it is a case-insensitive substring match on
#              the ISO filename (e.g. -l Spanish).
#   -d         Download the ISO instead of just printing the link.
#   --list-languages  List available ISO files for the chosen edition and exit.
#   --url-only  Print the download URL without downloading the file.
#   --no-check  Skip archive.org connectivity check.
#   --cookie <str>  Browser cookie string for manual anti-bot bypass.
#   --help      Show this help message and exit.
set -uo pipefail

# ── Defaults ─────────────────────────────────────────────────────────────────
API_BASE="${WISODOWN_API:-http://localhost:3000}"
EDITION=""
LANGUAGE=""
DO_DOWNLOAD=false
LIST_LANGUAGES=false
URL_ONLY=false
CHECK_CONNECTIVITY=true

# ── Helpers ──────────────────────────────────────────────────────────────────
cyan()  { printf '\033[36m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }
red()   { printf '\033[31m%s\033[0m\n' "$*"; }
dim()   { printf '\033[2m%s\033[0m\n' "$*"; }
bold()  { printf '\033[1m%s\033[0m\n' "$*"; }

# ── Parse CLI options ────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        -a|--api-base)      API_BASE="$2"; shift 2 ;;
        -e|--edition)       EDITION="$2"; shift 2 ;;
        -l|--language)      LANGUAGE="$2"; shift 2 ;;
        -d|--download)      DO_DOWNLOAD=true; shift ;;
        --list-languages)   LIST_LANGUAGES=true; shift ;;
        --url-only)         URL_ONLY=true; shift ;;
        --no-check)         CHECK_CONNECTIVITY=false; shift ;;
        --cookie)           API_COOKIE="$2"; shift 2 ;;
        -h|--help)          head -20 "$0" | tail -16; exit 0 ;;
        *)                  red "Unknown option: $1"; exit 1 ;;
    esac
    shift
done

# ── Archive.org connectivity check (best-effort, non-fatal) ──────────────────
if [[ "$CHECK_CONNECTIVITY" == true ]]; then
    if ! curl -s -f --max-time 5 "https://archive.org" > /dev/null 2>&1; then
        printf '\033[33m%s\033[0m\n' "⚠ archive.org seems unreachable – continuing anyway (use --no-check to skip)."
    fi
fi

# ── API helper ───────────────────────────────────────────────────────────────
api_get() {
    local url="$1"
    local body
    if ! body=$(curl -sf -m 15 "$url" 2>/dev/null); then
        red "[ERROR] Could not reach the API at $API_BASE" >&2
        exit 1
    fi
    API_OUT="$body"
}

# ── Edition catalogue (Microsoft + archive/or) ──────────────────────────────
EDITIONS=(
    "x64"           "Windows 11 (x64)"
    "arm64"         "Windows 11 (ARM64)"
    "win10"         "Windows 10 (22H2)"
    "win11-cn-home" "Windows 11 Home (China)"
    "win11-cn-pro"  "Windows 11 Pro (China)"
    "win7"          "Windows 7 (archive.org) — English only"
    "win8"          "Windows 8 (archive/or) — English only"
    "win8.1"        "Windows 8.1 (archive/or) — multilingual"
)

KEYS=()
LABELS=()
i=0
while [[ $i -lt ${#EDITIONS[@]} ]]; do
    KEYS+=("${EDITIONS[$i]}")
    LABELS+=("${EDITIONS[$((i+1))]}")
    i=$((i+2))
done

# ── Validate edition (if provided via flag) ───────────────────────────────────
if [[ -n "$EDITION" ]]; then
    MATCHED=false
    for k in "${KEYS[@]}"; do
        if [[ "${k,,}" == "${EDITION,,}" ]]; then
            MATCHED=true
            EDITION="$k"   # normalize case
            break
        fi
    done
    if [[ "$MATCHED" == false ]]; then
        red "Unknown edition: $EDITION."
        echo "Valid editions: ${KEYS[*]}"
        exit 1
    fi
    # If edition was provided via flag, skip the interactive menu
    : # edition already set, continue below
fi

# ── 1. Select edition ────────────────────────────────────────────────────────
# If edition was NOT provided via flag, show interactive menu
if [[ -z "$EDITION" ]]; then
    echo ""
    bold "Windows ISO Downloader — Select edition"
    echo "========================================="
    for i in "${!KEYS[@]}"; do
        printf "  [%d] %s\n" $((i+1)) "${LABELS[$i]}"
    done
    echo ""
    while true; do
        read -rp "Select edition (1-${#KEYS[@]}): " choice
        if [[ "$choice" =~ ^[0-9]+$ ]] && (( choice >= 1 && choice <= ${#KEYS[@]} )); then
            EDITION="${KEYS[$((choice-1))]}"
            break
        fi
        red "Invalid choice – enter a number 1-${#KEYS[@]}."
    done
fi

# ── 2. Fetch languages ───────────────────────────────────────────────────────
cyan "-> Fetching languages for '$EDITION'..."
api_get "$API_BASE/api/skus?edition=$EDITION"
skus_json="$API_OUT"

if echo "$skus_json" | jq -e '.error' &>/dev/null; then
    red "[ERROR] $(jq_val '.error' "$skus_json")"
    exit 1
fi

sku_count=$(jq_val 'length' "$skus_json")

# Determine language selection mode based on edition
is_legacy=false
case "$EDITION" in
    win7|win8)      is_legacy=true ;;
esac

if [[ -z "$LANGUAGE" ]]; then
    echo ""
    if [[ "$is_legacy" == true ]]; then
        LANGUAGE="English"
        echo "  The selected edition only ships English ISOs."
    else
        for ((i = 0; i < sku_count; i++)); do
            loc=$(jq_val '.[$i].LocalizedLanguage' "$skus_json")
            lang=$(jq_val '.[$i].Language' "$skus_json")
            printf "  [%d] %s (%s)\n" $((i+1)) "$loc" "$lang"
        done
        echo ""
        while true; do
            read -rp "Select language (1-$sku_count): " choice
            if [[ "$choice" -ge 1 && "$choice" -le "$sku_count" ]]; then
                LANGUAGE=$(jq_val ".[$((choice-1))].Language" "$skus_json")
                break
            fi
            red "Invalid choice – enter 1-$sku_count."
        done
    fi
fi

# ── 3. Get download links ────────────────────────────────────────────────────
cyan "-> Generating download link..."
api_get "$API_BASE/api/links?edition=$EDITION&language=$(printf '%s' "$LANGUAGE" | jq -sRr @uri)"
links_json="$API_OUT"

if echo "$links_json" | jq -e '.error' &>/dev/null; then
    red "[ERROR] $(jq_val '.error' "$links_json")"
    exit 1
fi

loc_lang=$(jq_val '.localizedLanguage' "$links_json")
filename=$(jq_val '.filename' "$links_json")
expires=$(jq_val '.expiresAt' "$links_json")

echo ""
green "[OK] $loc_lang"
[[ -n "$filename" ]] && dim "  File: $filename"
[[ -n "$expires" ]] && dim "  Expires: $expires"
echo ""

# ── 4. Handle output mode ───────────────────────────────────────────────────
dl_count=$(jq_val '.downloads | length' "$links_json")

# --url-only: just print the first URL and exit
if [[ "$URL_ONLY" == true ]]; then
    if [[ "$dl_count" -gt 0 ]]; then
        url=$(jq_val '.downloads[0].url' "$links_json")
        echo "$url"
    else
        red "No download links available."
        exit 1
    fi
    exit 0
fi

# --list-languages: list ISO files
if [[ "$LIST_LANGUAGES" == true ]]; then
    echo ""
    echo "=== Available ISOs for $EDITION ==="
    if [[ "$is_legacy" == true && "$EDITION" == "win7" ]]; then
        echo "  Win7_Ult_SP1_English_x32.iso   2.4 GiB"
        echo "  Win7_Ult_SP1_English_x64.iso   3.1 GiB"
    elif [[ "$is_legacy" == true && "$EDITION" == "win8" ]]; then
        echo "  en_windows_8_x64_dvd_915440.iso   3.3 GiB"
        echo "  en_windows_8_x86_dvd_915417.iso   2.5 GiB"
    elif [[ "$EDITION" == "win8.1" ]]; then
        echo "  (First 12 of 74 available ISOs for win8.1)"
        echo "  Win8.1_Arabic_x32.iso          3.0 GiB"
        echo "  Win8.1_Arabic_x64.iso          4.0 GiB"
        echo "  Win8.1_BrazilianPortuguese_x32.iso   3.0 GiB"
        echo "  Win8.1_BrazilianPortuguese_x64.iso   4.0 GiB"
        echo "  Win8.1_Bulgarian_x32.iso       2.9 GiB"
        echo "  Win8.1_Chinese(Simplified)_x32.iso   3.1 GiB"
        echo "  Win8.1_Chinese(Simplified)_x64.iso   4.2 GiB"
        echo "  Win8.1_Chinese(Traditional)_x32.iso    3.1 GiB"
        echo "  Win8.1_Chinese(Traditional)_x64.iso    4.2 GiB"
        echo "  Win8.1_Croatian_x32.iso        2.8 GiB"
        echo "  Win8.1_Croatian_x64.iso        3.9 GiB"
    else
        for ((i = 0; i < sku_count; i++)); do
            lang=$(jq_val '.[$i].Language' "$skus_json")
            echo "  $lang (id: $(jq_val '.[$i].id' "$skus_json"))"
        done
    fi
    echo ""
    exit 0
fi

# ── 5. Download if requested ────────────────────────────────────────────────
if [[ "$DO_DOWNLOAD" == true && "$dl_count" -gt 0 ]]; then
    url=$(jq_val '.downloads[0].url' "$links_json")
    out=${filename:-windows.iso}
    cyan "-> Downloading $out..."
    if [[ -n "$API_COOKIE" ]]; then
        dim "  (Cookie supplied but not injected into this curl call)"
    fi
    curl -L -o "$out" "$url"
    green "[OK] Saved to $out"
elif [[ "$DO_DOWNLOAD" == true && "$dl_count" -eq 0 ]]; then
    red "No download links available for the selected edition/language."
    exit 1
fi

echo ""
green "Done."