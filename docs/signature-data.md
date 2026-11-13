# Signature data

What helperpad sends back for `SignatureCapture`:

| Field | What helperpad sends | Based on |
|---|---|---|
| `imageData` | Base64 JPG or PNG (`imageFormat` 1 or 2), sized `imageX` × `imageY` | Topaz integration guide |
| `sigString` | Topaz SigString as uppercase hex, in `sigCompressionMode` 0 (default) through 8. With `encryptionMode` 1 (DES), 2 or 3 (SAFER), or 4 (DES with a SHA-256 receipt) it is encrypted with `encryptionKey` | `SigWriter` and `SigCrypt` in SigPlus Java 2.68 |
| `rawData` | `"ENC"` (default): the `$VPAD$2.0$` blob with points AES-256-CBC encrypted and the key RSA-encrypted to eSign Emcee's SigExtract certificate. `"SIG"`: plain .sig text | `Capture.GetSignatureData` in Topaz.SigPlusExtLite.exe 3.2.17.2 |
| `padInfo` | `Topaz T-L460` + a SHA-1 serial | Same |

## How it was checked

The `sigString` is byte for byte what Topaz's Windows SigPlus control writes. The real control (`SigPlus.ocx`) was run under Wine and its output compared with helperpad's for clear text and every encryption mode, at compression 0, 1, 2, 4, and 8: 50 of 50 were identical. Topaz's Java `SigPlus` class also reads them back with the same strokes and points, and a real signature from Topaz's demo page passed the same checks. The `"SIG"` rawData loads there too. The `"ENC"` rawData can only be decrypted by eSign Emcee; tests check its layout by decrypting it with a test key.

## Limits

- The T-L460-HSB-R doesn't answer Topaz's model and serial query and has no USB serial, so `padInfo` and the rawData owner use a fixed stand-in (model `T-L460`, serial `helperpad`).
