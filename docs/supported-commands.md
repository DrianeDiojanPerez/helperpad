# Supported commands

These are the commands a website can send through the SigPlusExtLite extension, and what helperpad answers.

| Command | Response |
|---|---|
| `SignatureCapture` | Signing window, then `imageData`, `sigString`, `rawData`, and `padInfo` (see [Signature data](signature-data.md)) |
| `GetVersionInfo` | `spelVersion` 3.1.0.0 |
| `GetDeviceStatus` | `1` when a pad is plugged in, otherwise `0` |
| `Connect`, `Disconnect`, `GetActiveXVersionInfo` | Success |
| Anything else | `responseType: -1` (unsupported) |
