# Commands

## helperpad

These work wherever helperpad is installed:

| Command | What it does |
|---|---|
| `helperpad window` | Open the signing window to test the pad |
| `helperpad status` | Show the binary, browser registration, extension, pad, udev rule, and log |
| `helperpad logs` | Watch the log live while you sign (Ctrl+C to stop) |
| `helperpad update` | Download and install the newest release |
| `helperpad update --check` | Only say whether a newer version exists |
| `helperpad uninstall` | Remove helperpad (add `--yes` to skip the question) |
| `helperpad version` | Print the version |
| `helperpad help` | List the commands |

## make

These need the source folder. Every command also works with [`just`](https://github.com/casey/just): `just install`, `just logs`, and so on. The `justfile` stands on its own and doesn't need `make`.

| Command | What it does |
|---|---|
| `make` | List the commands |
| `make install` | Set up everything: tools, pad access, helperpad, browser registration, extension check |
| `make update` | Same as `helperpad update` |
| `make uninstall` | Same as `helperpad uninstall --yes` |
| `make status` | Same as `helperpad status` |
| `make logs` | Same as `helperpad logs` |
| `make window` | Same as `helperpad window` |
| `make check-sigstring` | Check a SigString with Topaz's library (`FILE=`, `MODE=`, `OUT=`, `ENC=`, `KEY=`) |
| `make test` / `make lint` / `make build` | Tests, clippy, release build |
| `make add-version` / `make dry-release` / `make release` | See [Releasing](releasing.md) |

## Checking a SigString

`make check-sigstring FILE=sigstring.txt` loads a SigString into Topaz's own SigPlus Java library and prints what it reads back. Run it without `FILE` to paste one instead.

- `MODE=` is the compression mode the string was made with (0 by default).
- `OUT=drawing.png` saves Topaz's drawing of it.
- `ENC=` and `KEY=` are the encryption mode and key, for an encrypted string.

`just check-sigstring` takes plain arguments in the same order: `just check-sigstring ~/sigstring.txt 0 drawing.png`.
