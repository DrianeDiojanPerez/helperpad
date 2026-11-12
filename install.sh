#!/usr/bin/env bash
# Sets up everything a Topaz T-L460-HSB-R pad needs to sign on websites in Chrome on Linux:
# build tools, pad access (the Linux "driver" part), helperpad, and the Topaz extension.
# Safe to rerun; each step skips what's already done. Asks for your sudo password when needed.
set -euo pipefail

cd "$(dirname "$0")"

UDEV_RULE=/etc/udev/rules.d/70-topaz-sigpad.rules
EXTENSION_ID=dhcpobccjkdnmibckgpejmbpmpembgco
HOST_NAME=com.topaz.sigplusextlite.win.2
BIN="$HOME/.local/bin/helperpad"
STORE_URL="https://chromewebstore.google.com/detail/topaz-sigplusextlite-exte/$EXTENSION_ID"
DEMO_URL="https://sigplusweb.com/sign_chrome_ff_sigplusextlite.html"
MIN_RUST_MINOR=95
RULES='SUBSYSTEM=="hidraw", ATTRS{idVendor}=="06a8", ATTRS{idProduct}=="0043", TAG+="uaccess"
SUBSYSTEM=="usbmisc", KERNEL=="hiddev*", ATTRS{idVendor}=="06a8", ATTRS{idProduct}=="0043", TAG+="uaccess"'

step() { echo; echo "==> [$1/6] $2"; }
ok() { echo "    $1"; }

package_manager() {
    for pm in pacman apt-get dnf zypper; do
        command -v "$pm" >/dev/null && { echo "$pm"; return; }
    done
    return 0
}

# Maps a generic need to each distro's package names.
packages_for() {
    case "$1:$2" in
        pacman:build) echo "base-devel curl" ;;
        apt-get:build) echo "build-essential curl ca-certificates" ;;
        dnf:build) echo "gcc make curl" ;;
        zypper:build) echo "gcc make curl" ;;
        pacman:gui) echo "libxkbcommon wayland libx11 libxcursor libxrandr libxi mesa" ;;
        apt-get:gui) echo "libxkbcommon0 libwayland-client0 libx11-6 libxcursor1 libxrandr2 libxi6 libgl1 libegl1" ;;
        dnf:gui) echo "libxkbcommon libwayland-client libX11 libXcursor libXrandr libXi mesa-libGL mesa-libEGL" ;;
        zypper:gui) echo "libxkbcommon0 libwayland-client0 libX11-6 libXcursor1 libXrandr2 libXi6 Mesa-libGL1 Mesa-libEGL1" ;;
    esac
}

install_packages() {
    local pm
    pm=$(package_manager)
    if [ -z "$pm" ]; then
        echo "    Unknown package manager. Install these yourself: $*" >&2
        exit 1
    fi
    ok "Installing with $pm: $*"
    case "$pm" in
        pacman) sudo pacman -S --needed --noconfirm "$@" ;;
        apt-get) sudo apt-get update -qq && sudo apt-get install -y "$@" ;;
        dnf) sudo dnf install -y "$@" ;;
        zypper) sudo zypper install -y "$@" ;;
    esac
}

libraries=$(ldconfig -p 2>/dev/null || true)
has_library() {
    grep -qF "$1" <<< "$libraries"
}

find_pad() {
    grep -l 000006A8 /sys/class/hidraw/*/device/uevent 2>/dev/null | head -1 | cut -d/ -f5 || true
}

step 1 "Checking build tools and libraries"
pm=$(package_manager)
if ! command -v cc >/dev/null || ! command -v make >/dev/null || ! command -v curl >/dev/null; then
    # shellcheck disable=SC2046
    install_packages $(packages_for "$pm" build)
else
    ok "C compiler, make, and curl found"
fi
[ -x "$HOME/.cargo/bin/cargo" ] && export PATH="$HOME/.cargo/bin:$PATH"
rust_minor=$(rustc --version 2>/dev/null | sed -n 's/^rustc 1\.\([0-9]*\).*/\1/p' || true)
if [ -z "$rust_minor" ] || [ "$rust_minor" -lt "$MIN_RUST_MINOR" ]; then
    if command -v rustup >/dev/null; then
        ok "Rust is older than 1.$MIN_RUST_MINOR, updating it with rustup"
        rustup update stable
    else
        ok "Rust 1.$MIN_RUST_MINOR or newer not found, installing it with rustup (https://rustup.rs)"
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    fi
    export PATH="$HOME/.cargo/bin:$PATH"
fi
ok "$(cargo --version)"
if ! has_library libxkbcommon.so.0 || ! { has_library libwayland-client.so.0 || has_library libX11.so.6; } || ! { has_library libEGL.so.1 || has_library libGL.so.1; }; then
    # shellcheck disable=SC2046
    install_packages $(packages_for "$pm" gui)
else
    ok "Window libraries found"
fi

step 2 "Giving your user access to the pad"
if [ -f "$UDEV_RULE" ] && [ "$(cat "$UDEV_RULE")" = "$RULES" ]; then
    ok "udev rule already in place: $UDEV_RULE"
else
    ok "Writing $UDEV_RULE (needs sudo)"
    echo "$RULES" | sudo tee "$UDEV_RULE" >/dev/null
    sudo udevadm control --reload
    sudo udevadm trigger
    ok "udev rule installed"
fi
ok "The pad itself needs no driver: the Linux kernel's built-in HID driver runs it"

step 3 "Building and installing helperpad"
cargo build --release
install -Dm755 target/release/helperpad "$BIN"
ok "Installed $BIN"
for browser in google-chrome chromium; do
    [ -d "$HOME/.config/$browser" ] || continue
    manifest="$HOME/.config/$browser/NativeMessagingHosts/$HOST_NAME.json"
    mkdir -p "$(dirname "$manifest")"
    cat > "$manifest" <<EOF
{
  "name": "$HOST_NAME",
  "description": "helperpad: Topaz SigPlusExtLite host for Linux",
  "path": "$BIN",
  "type": "stdio",
  "allowed_origins": ["chrome-extension://$EXTENSION_ID/"]
}
EOF
    ok "Registered with Chrome: $manifest"
done

step 4 "Checking the pad"
dev=$(find_pad)
if [ -z "$dev" ] || [ ! -r "/dev/$dev" ]; then
    if [ -z "$dev" ]; then
        echo "    Plug in the pad now."
    else
        echo "    The pad was plugged in before the udev rule existed. Unplug it and plug it back in now."
    fi
    if [ -t 0 ]; then
        echo "    Waiting up to 60 seconds..."
        for _ in $(seq 60); do
            sleep 1
            dev=$(find_pad)
            [ -n "$dev" ] && [ -r "/dev/$dev" ] && break
        done
    fi
fi
if [ -n "$dev" ] && [ -r "/dev/$dev" ]; then
    ok "Pad found at /dev/$dev and readable"
else
    ok "Pad not ready yet. Plug it in (or replug it), then run 'make status' to check."
fi

step 5 "Checking Chrome and the Topaz extension"
chrome=""
for name in google-chrome-stable google-chrome chromium chromium-browser; do
    command -v "$name" >/dev/null && { chrome=$name; break; }
done
if [ -z "$chrome" ]; then
    ok "Chrome isn't installed. Install Google Chrome, then run this script again."
elif [[ "$(readlink -f "$(command -v "$chrome")")" == /snap/* ]]; then
    ok "Only the Snap version of Chromium is installed. Its sandbox blocks helperpad."
    ok "Install Google Chrome from https://www.google.com/chrome/ (the .deb), then run this script again."
elif compgen -G "$HOME/.config/*/*/Extensions/$EXTENSION_ID" >/dev/null; then
    ok "Topaz SigPlusExtLite extension installed"
else
    ok "Opening the Chrome Web Store so you can add the Topaz SigPlusExtLite extension:"
    ok "$STORE_URL"
    "$chrome" "$STORE_URL" >/dev/null 2>&1 &
fi

step 6 "Summary"
make --no-print-directory status
echo
echo "Done. Next:"
echo "  1. Close Chrome completely and open it again."
echo "  2. Sign on Topaz's demo page: $DEMO_URL"
echo "  3. If something's off, run 'make status' and 'make logs'."
