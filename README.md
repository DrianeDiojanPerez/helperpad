<p align="center"><img src="assets/banner.png" alt="helperpad: Topaz signature pads in Chrome on Linux" width="100%"></p>

## Install

```
curl -fsSL https://raw.githubusercontent.com/DrianeDiojanPerez/helperpad/master/install.sh | sh
```

This downloads the newest release for your system, checks it against the release's checksum, and installs it with `apt-get`, `dnf`, `zypper`, or `pacman` (it asks for your sudo password). On other systems it puts the binary in `~/.local/bin` and sets up Chrome and the pad's udev rule itself. Then replug the pad, add the [Topaz SigPlusExtLite extension](https://chromewebstore.google.com/detail/topaz-sigplusextlite-exte/dhcpobccjkdnmibckgpejmbpmpembgco) in Chrome, and restart Chrome.

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
4. Install the [Topaz SigPlusExtLite extension](https://chromewebstore.google.com/detail/topaz-sigplusextlite-exte/dhcpobccjkdnmibckgpejmbpmpembgco) in Chrome.
5. Close Chrome completely and open it again. Do this after every install, because a helper that's already running keeps the old version.

Run `make status` to check that everything is in place.

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

## Make commands

Every command also works with [`just`](https://github.com/casey/just): `just install`, `just logs`, and so on. The `justfile` stands on its own and doesn't need `make`. `just check-sigstring` takes plain arguments: `just check-sigstring ~/sigstring.txt 0 drawing.png`.

| Command | What it does |
|---|---|
| `make` | List the commands |
| `make install` | Set up everything: tools, pad access, helperpad, Chrome registration, extension check |
| `make update` | Download and install the newest release |
| `make uninstall` | Remove the binary and the Chrome registration (keeps the log) |
| `make status` | Show the binary, Chrome registration, extension, pad, udev rule, and log |
| `make logs` | Watch the log live while you sign (Ctrl+C to stop) |
| `make window` | Open the signing window to test the pad |
| `make check-sigstring` | Check a SigString with Topaz's library (`FILE=`, `MODE=`, `OUT=`) |
| `make test` / `make lint` / `make build` | Tests, clippy, release build |

## Logs and where things live

| What | Where |
|---|---|
| Log | `~/.local/state/helperpad/helperpad.log` |
| Program | `~/.local/bin/helperpad` |
| Chrome registration | `~/.config/google-chrome/NativeMessagingHosts/com.topaz.sigplusextlite.win.2.json` (and the same under `chromium/`) |
| Last update check | `~/.local/state/helperpad/update-check` |
| Pad permissions | `/etc/udev/rules.d/70-topaz-sigpad.rules` |
| Topaz SDK for `check-sigstring` | `~/.cache/helperpad/topaz-check/` (downloaded on first use) |

Every line in the log has the local time, the helper's process ID, and what happened:

```
2026-11-12 09:47:23 [571829] started by chrome-extension://dhcpobccjkdnmibckgpejmbpmpembgco/
2026-11-12 09:47:23 [571829] request SignatureCapture
2026-11-12 09:47:31 [571829] response SignatureCapture: responseType=0 isSigned=true errorMsg=""
```

- `started by …` means Chrome launched the helper. Chrome keeps it running until the tab closes, so one process ID can cover many signatures.
- `request …` is each command the website sent through the extension.
- `response …` is what helperpad answered. `isSigned=true` means a signature went back to the page; `errorMsg` says why one didn't.
- `updated from … to …` and `update to … failed: …` record updates.
- `sigString left empty …` and `rawData left empty …` explain a missing field.

## Supported commands

| Command | Response |
|---|---|
| `SignatureCapture` | Signing window, then `imageData`, `sigString`, `rawData`, and `padInfo` (see below) |
| `GetVersionInfo` | `spelVersion` 3.1.0.0 |
| `GetDeviceStatus` | `1` when a pad is plugged in, otherwise `0` |
| `Connect`, `Disconnect`, `GetActiveXVersionInfo` | Success |
| Anything else | `responseType: -1` (unsupported) |

## Signature data

| Field | What helperpad sends | Based on |
|---|---|---|
| `imageData` | Base64 JPG or PNG (`imageFormat` 1 or 2), sized `imageX` × `imageY` | Topaz integration guide |
| `sigString` | Topaz SigString as uppercase hex, in `sigCompressionMode` 0 (default) through 8 | `SigWriter` in SigPlus Java 2.68 |
| `rawData` | `"ENC"` (default): the `$VPAD$2.0$` blob with points AES-256-CBC encrypted and the key RSA-encrypted to eSign Emcee's SigExtract certificate. `"SIG"`: plain .sig text | `Capture.GetSignatureData` in Topaz.SigPlusExtLite.exe 3.2.17.2 |
| `padInfo` | `Topaz T-L460` + a SHA-1 serial | Same |

Every `sigString` mode was checked by loading it into Topaz's own Java `SigPlus` class, which read back the same strokes and points, and a real signature from Topaz's demo page passed the same check. The `"SIG"` rawData loads there too. The `"ENC"` rawData can only be decrypted by eSign Emcee; tests check its layout by decrypting it with a test key.

## Limits

- `sigString` with `encryptionMode` 1–3 (DES or SAFER) comes back empty. Only clear text (mode 0) is supported.
- The pad doesn't report its model number or serial over Linux HID, so `padInfo` and the rawData owner use a fixed stand-in (model `T-L460`, serial `helperpad`).
- Windows builds its SigString with the SigPlus ActiveX control. helperpad follows the Java SDK's format, which Topaz describes as the same, but it hasn't been compared against a Windows-made SigString.
- Custom signing windows, LCD pads, and GemView tablets aren't supported.
- Chrome and Chromium only. Firefox uses a different host manifest and extension ID.
