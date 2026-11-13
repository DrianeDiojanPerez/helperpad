<p align="center"><img src="assets/banner.png" alt="helperpad: Topaz signature pads in Chrome and Firefox on Linux" width="100%"></p>

## Install

```
curl -fsSL https://raw.githubusercontent.com/DrianeDiojanPerez/helperpad/master/install.sh | sh
```

This downloads the newest release for your system, checks it against the release's checksum, and installs it with `apt-get`, `dnf`, `zypper`, or `pacman` (it asks for your sudo password). On other systems it puts the binary in `~/.local/bin` and sets up Chrome and the pad's udev rule itself. Then replug the pad, add the Topaz SigPlusExtLite extension for [Chrome](https://chromewebstore.google.com/detail/topaz-sigplusextlite-exte/dhcpobccjkdnmibckgpejmbpmpembgco) or [Firefox](https://addons.mozilla.org/firefox/addon/topaz-sigplusextlite-extension/), and restart the browser.

### Manual install

1. Give your user access to the pad (once, needs `sudo`):
   ```
   sudo tee /etc/udev/rules.d/70-topaz-sigpad.rules >/dev/null <<'EOF'
   SUBSYSTEM=="hidraw", ATTRS{idVendor}=="06a8", ATTRS{idProduct}=="0043", TAG+="uaccess"
   SUBSYSTEM=="usbmisc", KERNEL=="hiddev*", ATTRS{idVendor}=="06a8", ATTRS{idProduct}=="0043", TAG+="uaccess"
   EOF
   sudo udevadm control --reload && sudo udevadm trigger
   ```
   Then unplug the pad and plug it back in.
2. Build and install the binary:
   ```
   cargo build --release
   install -Dm755 target/release/helperpad ~/.local/bin/helperpad
   ```
3. Register it with Chrome by saving this as `~/.config/google-chrome/NativeMessagingHosts/com.topaz.sigplusextlite.win.2.json` (use `chromium` in the path for Chromium), with your own home folder in `path`:
   ```
   {
     "name": "com.topaz.sigplusextlite.win.2",
     "description": "helperpad: Topaz SigPlusExtLite host for Linux",
     "path": "/home/you/.local/bin/helperpad",
     "type": "stdio",
     "allowed_origins": ["chrome-extension://dhcpobccjkdnmibckgpejmbpmpembgco/"]
   }
   ```
4. For Firefox, save this as `~/.mozilla/native-messaging-hosts/com.topaz.sigplusextlite.win.2.json` (Zen uses `~/.zen/`, LibreWolf `~/.librewolf/`), again with your own home folder in `path`. Skip this step if you only use Chrome:
   ```
   {
     "name": "com.topaz.sigplusextlite.win.2",
     "description": "helperpad: Topaz SigPlusExtLite host for Linux",
     "path": "/home/you/.local/bin/helperpad",
     "type": "stdio",
     "allowed_extensions": ["sigplusextliteff@topazsystems.com"]
   }
   ```
5. Install the Topaz SigPlusExtLite extension for [Chrome](https://chromewebstore.google.com/detail/topaz-sigplusextlite-exte/dhcpobccjkdnmibckgpejmbpmpembgco) or [Firefox](https://addons.mozilla.org/firefox/addon/topaz-sigplusextlite-extension/).
6. Close the browser completely and open it again. Do this after every install, because a helper that's already running keeps the old version.

Run `helperpad status` to check that everything is in place.

## Update

helperpad updates itself. Either way works:

- **In the signing window:** when a newer release exists, a bar at the top says so. Press **Update now**. You can keep signing while it runs.
- **In a terminal:** `helperpad update` (or `make update`). `helperpad update --check` only tells you whether a newer version exists, and `helperpad version` prints the one you have.

The update downloads the release from GitHub, checks it against the release's SHA-256 checksum, and installs it the same way helperpad was installed:

| Installed with | What the update does |
|---|---|
| `make install` (binary in `~/.local/bin`) | Replaces the binary with the one from the release `tar.gz` |
| `.deb`, `.rpm`, or Arch package | Downloads the matching package and installs it with `apt-get`, `dnf`, `zypper`, or `pacman`. This needs your password: `sudo` asks in a terminal, and a `pkexec` prompt appears when you update from the signing window |

Afterwards close Chrome completely and open it again.

To find out about new releases, helperpad asks GitHub for the latest version number at most once a day, when the signing window opens. It sends nothing else.

## Test

- Open Topaz's demo at https://sigplusweb.com/sign_chrome_ff_sigplusextlite.html, click **Sign**, sign on the pad, and press **OK**. The signature shows up on the page, with the SigString in the left box.

## Docs

- [How it works](docs/how-it-works.md)
- [Other ways to install](docs/install.md): a package by hand, or from source
- [Commands](docs/commands.md): `helperpad` and `make`
- [Logs and where things live](docs/logs.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Uninstall](docs/uninstall.md)
- [Supported commands](docs/supported-commands.md): what a website can ask for
- [Signature data](docs/signature-data.md): what goes back to the page, how it was checked, and limits
- [Releasing](docs/releasing.md)

## License

MIT. See [LICENSE](LICENSE).
