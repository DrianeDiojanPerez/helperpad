bin          := home_directory() / ".local/bin/helperpad"
log          := env("XDG_STATE_HOME", home_directory() / ".local/state") / "helperpad/helperpad.log"

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

# Remove helperpad and its browser registration
uninstall:
    @test -x "{{bin}}" || { echo "helperpad isn't installed."; exit 1; }
    @"{{bin}}" uninstall --yes

# Show what's installed and whether the pad is found
status:
    @test -x "{{bin}}" || { echo "Not installed. Run just install first."; exit 1; }
    @"{{bin}}" status

# Watch the log live (Ctrl+C to stop)
logs:
    @mkdir -p "$(dirname "{{log}}")" && touch "{{log}}" && tail -f "{{log}}"

# Open the signing window to test the pad
window:
    @test -x "{{bin}}" || { echo "Run just install first."; exit 1; }
    @"{{bin}}" --sign

# Check a SigString with Topaz's own library; file "-" pastes it instead
check-sigstring file="-" mode="0" out="" enc="0" key="":
    @ENCRYPTION={{quote(enc)}} KEY={{quote(key)}} ./tools/check-sigstring.sh "{{file}}" "{{mode}}" {{ if out == "" { "" } else { quote(out) } }}

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
