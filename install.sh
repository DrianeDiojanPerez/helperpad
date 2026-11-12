#!/bin/sh
# Installs the newest helperpad release:
#   curl -fsSL https://raw.githubusercontent.com/DrianeDiojanPerez/helperpad/master/install.sh | sh
set -eu

REPO=DrianeDiojanPerez/helperpad
HOST_NAME=com.topaz.sigplusextlite.win.2
EXTENSION_ID=dhcpobccjkdnmibckgpejmbpmpembgco
STORE_URL="https://chromewebstore.google.com/detail/topaz-sigplusextlite-exte/$EXTENSION_ID"

say() { printf '%s\n' "$*"; }
fail() { say "helperpad install failed: $*" >&2; exit 1; }
has() { command -v "$1" >/dev/null 2>&1; }

as_root() {
    if [ "$(id -u)" -eq 0 ]; then
        "$@"
    elif has sudo; then
        sudo "$@"
    else
        fail "this step needs root and sudo isn't installed. Run the script as root."
    fi
}

# Without a package manager we know, the binary goes into the user's home instead.
install_from_archive() {
    tar -xzf "$1" -C "$tmp"
    install -Dm755 "$tmp/helperpad" "$HOME/.local/bin/helperpad"
    say "Installed $HOME/.local/bin/helperpad"

    for browser in google-chrome chromium; do
        [ -d "$HOME/.config/$browser" ] || continue
        manifest="$HOME/.config/$browser/NativeMessagingHosts/$HOST_NAME.json"
        mkdir -p "$(dirname "$manifest")"
        sed "s|/usr/bin/helperpad|$HOME/.local/bin/helperpad|" "$tmp/packaging/$HOST_NAME.json" > "$manifest"
        say "Registered with Chrome: $manifest"
    done

    say "Giving your user access to the pad (needs root)"
    as_root install -Dm644 "$tmp/packaging/70-topaz-sigpad.rules" /etc/udev/rules.d/70-topaz-sigpad.rules
    if has udevadm; then
        as_root udevadm control --reload || true
        as_root udevadm trigger || true
    fi
}

main() {
    [ "$(uname -s)" = Linux ] || fail "helperpad only runs on Linux."
    has curl || fail "curl is needed."

    case "$(uname -m)" in
        x86_64 | amd64) package_arch=amd64 archive_arch=x86_64 ;;
        aarch64 | arm64) package_arch=arm64 archive_arch=arm64 ;;
        *) fail "there are no builds for $(uname -m)." ;;
    esac

    latest=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest")
    version=${latest##*/v}
    case "$version" in
        '' | *[!0-9.]*) fail "couldn't find the newest release." ;;
    esac

    if has apt-get; then
        file="helperpad_${version}_linux_${package_arch}.deb"
    elif has dnf || has zypper; then
        file="helperpad_${version}_linux_${package_arch}.rpm"
    elif has pacman; then
        file="helperpad_${version}_linux_${package_arch}.pkg.tar.zst"
    else
        file="helperpad_Linux_${archive_arch}.tar.gz"
    fi

    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT
    chmod 755 "$tmp"
    base="https://github.com/$REPO/releases/download/v$version"

    say "Downloading helperpad $version ($file)"
    curl -fsSL -o "$tmp/$file" "$base/$file"
    curl -fsSL -o "$tmp/checksums.txt" "$base/checksums.txt"

    expected=$(awk -v name="$file" '$2 == name { print $1 }' "$tmp/checksums.txt")
    actual=$(sha256sum "$tmp/$file" | cut -d' ' -f1)
    [ -n "$expected" ] && [ "$expected" = "$actual" ] || fail "the download doesn't match its checksum."

    say "Installing"
    case "$file" in
        *.deb) as_root apt-get install -y "$tmp/$file" ;;
        *.rpm)
            if has dnf; then
                as_root dnf install -y "$tmp/$file"
            else
                as_root zypper --non-interactive install --allow-unsigned-rpm "$tmp/$file"
            fi
            ;;
        *.pkg.tar.zst) as_root pacman -U --noconfirm "$tmp/$file" ;;
        *) install_from_archive "$tmp/$file" ;;
    esac

    say ""
    say "helperpad $version is installed. Next:"
    say "  1. Unplug the pad and plug it back in."
    say "  2. Add the Topaz SigPlusExtLite extension in Chrome:"
    say "     $STORE_URL"
    say "  3. Close Chrome completely and open it again."
    say "Later, run 'helperpad update' to get new versions."
}

# Nothing runs until the whole script has arrived, so a cut-off download does nothing.
main "$@"
