bin          := home_directory() / ".local/bin/helperpad"
log          := env("XDG_STATE_HOME", home_directory() / ".local/state") / "helperpad/helperpad.log"
host_name    := "com.topaz.sigplusextlite.win.2"
extension_id := "dhcpobccjkdnmibckgpejmbpmpembgco"
store_url    := "https://chromewebstore.google.com/detail/topaz-sigplusextlite-exte/" + extension_id
browsers     := "google-chrome chromium"
udev_rule    := "/etc/udev/rules.d/70-topaz-sigpad.rules"

# List the commands
default:
    @just --list --unsorted

# Set up everything: tools, pad access, helperpad, extension
install:
    ./tools/install-from-source.sh

# Download and install the newest release
update:
    @test -x "{{bin}}" || { echo "Run just install first."; exit 1; }
    @"{{bin}}" update

# Remove the binary and the Chrome registration
uninstall:
    #!/usr/bin/env bash
    rm -fv "{{bin}}"
    for browser in {{browsers}}; do rm -fv "$HOME/.config/$browser/NativeMessagingHosts/{{host_name}}.json"; done
    echo "Logs kept in {{log}}. Run 'just logs-clear' or delete it to remove them."

# Show what's installed and whether the pad is found
status:
    #!/usr/bin/env bash
    if [ -x "{{bin}}" ]; then echo "binary:     {{bin}}"; else echo "binary:     not installed (run just install)"; fi
    for browser in {{browsers}}; do
        f="$HOME/.config/$browser/NativeMessagingHosts/{{host_name}}.json"
        [ -f "$f" ] && echo "chrome:     registered ($f)"
    done
    if ls -d "$HOME"/.config/google-chrome/*/Extensions/{{extension_id}} >/dev/null 2>&1; then
        echo "extension:  installed"
    else
        echo "extension:  not found in Chrome ({{store_url}})"
    fi
    dev=$(grep -l 000006A8 /sys/class/hidraw/*/device/uevent 2>/dev/null | head -1 | cut -d/ -f5)
    if [ -z "$dev" ]; then echo "pad:        not plugged in"
    elif [ -r "/dev/$dev" ]; then echo "pad:        /dev/$dev (readable)"
    else echo "pad:        /dev/$dev (no access: udev rule missing, replug the pad)"; fi
    if [ -f "{{udev_rule}}" ]; then echo "udev rule:  {{udev_rule}}"; else echo "udev rule:  missing"; fi
    if [ -f "{{log}}" ]; then echo "log:        {{log}} ($(wc -l < "{{log}}") lines)"; else echo "log:        {{log}} (empty)"; fi

# Show the last 50 log lines
logs:
    @if [ -f "{{log}}" ]; then tail -n 50 "{{log}}"; else echo "No log yet at {{log}}. It appears once Chrome starts helperpad."; fi

# Watch the log live (Ctrl+C to stop)
logs-follow:
    @mkdir -p "$(dirname "{{log}}")" && touch "{{log}}" && tail -f "{{log}}"

# Empty the log
logs-clear:
    @: > "{{log}}" 2>/dev/null; echo "Cleared {{log}}"

# Open the signing window to test the pad
window:
    @test -x "{{bin}}" || { echo "Run just install first."; exit 1; }
    @"{{bin}}" --sign

# Check a SigString with Topaz's own library; file "-" pastes it instead
check-sigstring file="-" mode="0" out="":
    @./tools/check-sigstring.sh "{{file}}" "{{mode}}" {{ if out == "" { "" } else { quote(out) } }}

# Run the tests
test:
    cargo test

# Run clippy
lint:
    cargo clippy

# Build without installing
build:
    cargo build --release

# Bump, commit, and tag a version, for example: just add-version 1.2.3
add-version version:
    ./tools/add-version.sh "{{version}}"

# Build every release package into dist/ without publishing
dry-release:
    goreleaser release --snapshot --clean

# Publish the tagged version to GitHub (needs GITHUB_TOKEN)
release:
    goreleaser release --clean
