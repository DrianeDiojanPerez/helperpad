# Installing

## The one-line installer

```
curl -fsSL https://raw.githubusercontent.com/DrianeDiojanPerez/helperpad/master/install.sh | sh
```

This downloads the newest release for your system, checks it against the release's checksum, and installs it with `apt-get`, `dnf`, `zypper`, or `pacman` (it asks for your sudo password). On other systems it puts the binary in `~/.local/bin` and sets up Chrome, Firefox, and the pad's udev rule itself. Then replug the pad, add the Topaz SigPlusExtLite extension for [Chrome](https://chromewebstore.google.com/detail/topaz-sigplusextlite-exte/dhcpobccjkdnmibckgpejmbpmpembgco) or [Firefox](https://addons.mozilla.org/firefox/addon/topaz-sigplusextlite-extension/), and restart the browser.

The manual steps are in the [README](../README.md#manual-install). The other two ways are below.

## Install a package by hand

Download the package for your system from the [releases page](https://github.com/DrianeDiojanPerez/helperpad/releases) and install it:

| System | Command |
|---|---|
| Ubuntu, Debian | `sudo apt install ./helperpad_*_linux_amd64.deb` |
| Fedora | `sudo dnf install ./helperpad_*_linux_amd64.rpm` |
| Arch | `sudo pacman -U helperpad_*_linux_amd64.pkg.tar.zst` |

The package installs `/usr/bin/helperpad`, the pad's udev rule, and the registration for Chrome, Chromium, and Firefox for all users. Afterwards replug the pad, add the Topaz SigPlusExtLite extension for [Chrome](https://chromewebstore.google.com/detail/topaz-sigplusextlite-exte/dhcpobccjkdnmibckgpejmbpmpembgco) or [Firefox](https://addons.mozilla.org/firefox/addon/topaz-sigplusextlite-extension/), and restart the browser. `arm64` packages and plain `tar.gz` archives are on the same page.

## Install from source

Plug in the pad and run:

```
make install
```

It does everything, skips whatever is already done, and asks for your sudo password when it needs it:

1. Installs missing build tools (C compiler, `make`), Rust (through rustup), and the window libraries, using pacman, apt, dnf, or zypper.
2. Gives your user access to the pad with a udev rule.
3. Builds helperpad, installs it to `~/.local/bin/helperpad`, and registers it with Chrome and Firefox.
4. Checks the pad is plugged in and readable, and waits up to 60 seconds for you to replug it if needed.
5. Opens the Chrome Web Store page for the Topaz SigPlusExtLite extension if it isn't installed.
6. Prints a summary (`helperpad status`).

Then close the browser completely and open it again. `just install` does the same thing. Run it again after you change the code; steps that are already done are skipped.

Tested on Arch and Ubuntu 24.04. On Ubuntu, use Google Chrome from Google's .deb (https://www.google.com/chrome/): Ubuntu's Chromium is a Snap, and its sandbox blocks helperpad. helperpad needs Rust 1.95 or newer; the script installs or updates it through rustup if needed.
