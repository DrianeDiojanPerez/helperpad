# Logs and where things live

| What | Where |
|---|---|
| Log | `~/.local/state/helperpad/helperpad.log` |
| Program | `~/.local/bin/helperpad`, or `/usr/bin/helperpad` from a package |
| Chrome registration | `~/.config/google-chrome/NativeMessagingHosts/com.topaz.sigplusextlite.win.2.json` (and the same under `chromium/`) |
| Firefox registration | `~/.mozilla/native-messaging-hosts/com.topaz.sigplusextlite.win.2.json` (Zen uses `~/.zen/`, LibreWolf `~/.librewolf/`) |
| Registration from a package | `/etc/opt/chrome/native-messaging-hosts/`, `/etc/chromium/native-messaging-hosts/`, `/usr/lib/mozilla/native-messaging-hosts/` |
| Last update check | `~/.local/state/helperpad/update-check` |
| Pad permissions | `/etc/udev/rules.d/70-topaz-sigpad.rules`, or `/usr/lib/udev/rules.d/` from a package |
| Topaz SDK for `check-sigstring` | `~/.cache/helperpad/topaz-check/` (downloaded on first use) |

`helperpad logs` follows the log. Every line has the local time, the helper's process ID, and what happened:

```
2026-11-12 09:47:23 [571829] started by chrome-extension://dhcpobccjkdnmibckgpejmbpmpembgco/
2026-11-12 09:47:23 [571829] request SignatureCapture
2026-11-12 09:47:31 [571829] response SignatureCapture: responseType=0 isSigned=true errorMsg=""
```

- `started by …` means the browser launched the helper. Chrome shows its extension address there and Firefox shows `sigplusextliteff@topazsystems.com`. The browser keeps the helper running until the tab closes, so one process ID can cover many signatures.
- `request …` is each command the website sent through the extension.
- `response …` is what helperpad answered. `isSigned=true` means a signature went back to the page; `errorMsg` says why one didn't.
- `updated from … to …` and `update to … failed: …` record updates.
- `sigString left empty …` and `rawData left empty …` explain a missing field.
