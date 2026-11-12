# helperpad

A Linux stand-in for Topaz's Windows-only SigPlusExtLite helper. With it, websites that use the Topaz SigPlusExtLite browser extension can capture signatures from a Topaz HSB pad on Linux.

```
Website → SigPlusExtLite extension → helperpad → Topaz pad (/dev/hidrawN)
```

Chrome starts `helperpad` as the native messaging host `com.topaz.sigplusextlite.win.2`. When a page asks for a signature, helperpad opens a signing window with live ink and Clear / OK / Cancel buttons, then sends the signature back to the page.

Built for the Topaz T-L460-HSB-R (USB ID `06a8:0043`).

## Install

1. Give your user access to the pad (once, needs `sudo`):
   ```
   sudo tee /etc/udev/rules.d/70-topaz-sigpad.rules >/dev/null <<'EOF'
   SUBSYSTEM=="hidraw", ATTRS{idVendor}=="06a8", ATTRS{idProduct}=="0043", TAG+="uaccess"
   SUBSYSTEM=="usbmisc", KERNEL=="hiddev*", ATTRS{idVendor}=="06a8", ATTRS{idProduct}=="0043", TAG+="uaccess"
   EOF
   sudo udevadm control --reload && sudo udevadm trigger
   ```
   Then unplug the pad and plug it back in.
2. Build, install, and register with Chrome:
   ```
   make install
   ```
3. Install the [Topaz SigPlusExtLite extension](https://chromewebstore.google.com/detail/topaz-sigplusextlite-exte/dhcpobccjkdnmibckgpejmbpmpembgco) in Chrome.
4. Close Chrome completely and open it again. Do this after every `make install`, because a helper that's already running keeps the old version.

Run `make status` to check that everything is in place.

## Test

- Open Topaz's demo at https://sigplusweb.com/sign_chrome_ff_sigplusextlite.html, click **Sign**, sign on the pad, and press **OK**. The signature shows up on the page, with the SigString in the left box.
- `make window` opens only the signing window, without Chrome.
- `make check-sigstring FILE=sigstring.txt` loads a SigString into Topaz's own SigPlus Java library and prints what it reads back. Run it without `FILE` to paste one instead. Add `OUT=drawing.png` to save Topaz's drawing of it.

## Make commands

Every command also works with [`just`](https://github.com/casey/just): `just install`, `just logs`, and so on. The `justfile` stands on its own and doesn't need `make`. `just check-sigstring` takes plain arguments: `just check-sigstring ~/sigstring.txt 0 drawing.png`.

| Command | What it does |
|---|---|
| `make` | List the commands |
| `make install` | Build, install the binary, and register it with Chrome |
| `make uninstall` | Remove the binary and the Chrome registration (keeps the log) |
| `make status` | Show the binary, Chrome registration, extension, pad, udev rule, and log |
| `make logs` | Show the last 50 log lines |
| `make logs-follow` | Watch the log live while you sign (Ctrl+C to stop) |
| `make logs-clear` | Empty the log |
| `make window` | Open the signing window to test the pad |
| `make check-sigstring` | Check a SigString with Topaz's library (`FILE=`, `MODE=`, `OUT=`) |
| `make test` / `make lint` / `make build` | Tests, clippy, release build |

## Logs and where things live

| What | Where |
|---|---|
| Log | `~/.local/state/helperpad/helperpad.log` |
| Program | `~/.local/bin/helperpad` |
| Chrome registration | `~/.config/google-chrome/NativeMessagingHosts/com.topaz.sigplusextlite.win.2.json` (and the same under `chromium/`) |
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

## Troubleshooting

Start with `make status`, then `make logs`.

- **Nothing in the log after clicking Sign:** Chrome isn't starting helperpad. Check `make status` shows it registered and the extension installed, then restart Chrome fully.
- **"could not open the signing window: No such file or directory":** a helper from before your last `make install` is still running. Restart Chrome.
- **"No Topaz signature pad found":** the pad isn't plugged in. Check `lsusb | grep 06a8`.
- **"Can't read the pad" or `make status` says no access:** the udev rule is missing or not applied. Do step 1 of Install, then replug the pad.
- **Page says the extension isn't installed:** install it from the link in step 3 and reload the page.

## Uninstall

```
make uninstall
```

The log stays in `~/.local/state/helperpad/`. Delete that folder to remove it too.
