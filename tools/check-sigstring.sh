#!/usr/bin/env bash
# Loads a SigString into Topaz's own SigPlus Java library and reports what it reads back.
# Usage: tools/check-sigstring.sh <file-with-sigstring-hex | -> [compression-mode] [output.png]
# Pass "-" (or nothing) to paste the SigString instead, then press Ctrl+D.
# For an encrypted SigString, set ENCRYPTION (1-4) and KEY to what the website sent.
set -euo pipefail

source=${1:--}
mode=${2:-0}
image=${3:+$(realpath -m "$3")}

if [ "$source" = "-" ]; then
    input=$(mktemp)
    trap 'rm -f "$input"' EXIT
    [ -t 0 ] && echo "Paste the SigString, then press Enter and Ctrl+D:" >&2
    sed 's/^SigString: *//' > "$input"
elif [ -f "$source" ]; then
    input=$(realpath "$source")
else
    echo "No file at $(realpath -m "$source")." >&2
    echo "Save the SigString to a file first, or run with \"-\" to paste it." >&2
    exit 1
fi

cache="${XDG_CACHE_HOME:-$HOME/.cache}/helperpad/topaz-check"
sdk="$cache/sigplusjava2_68"
url="https://www.topazsystems.com/Software/download/java/sigplusjava2_68.zip"
sha256="6a55baf2cd4e29047b2cb6679c343887dc279dc3c5f00c819dda744147999d6e"

if [ ! -f "$sdk/SigPlus2_68.jar" ]; then
    mkdir -p "$cache"
    curl -sSL -A "Mozilla/5.0" -o "$cache/sdk.zip" "$url"
    echo "$sha256  $cache/sdk.zip" | sha256sum -c --quiet
    bsdtar -xf "$cache/sdk.zip" -C "$cache"
fi

classpath="$sdk/SigPlus2_68.jar:$sdk/RXTXcomm.jar"
javac -nowarn -cp "$classpath" -d "$cache/classes" "$(dirname "$0")/topaz-check/Check.java" 2>/dev/null
java -cp "$classpath:$cache/classes" Check "$input" "$mode" ${image:+"$image"}
