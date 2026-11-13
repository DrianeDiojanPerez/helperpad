# Updating

helperpad updates itself. Either way works:

- **In the signing window:** when a newer release exists, a bar at the top says so. Press **Update now**. You can keep signing while it runs.
- **In a terminal:** `helperpad update` (or `make update`). `helperpad update --check` only tells you whether a newer version exists, and `helperpad version` prints the one you have.

The update downloads the release from GitHub, checks it against the release's SHA-256 checksum, and installs it the same way helperpad was installed:

| Installed with | What the update does |
|---|---|
| The installer's `tar.gz` fallback or `make install` (binary in `~/.local/bin`) | Replaces the binary with the one from the release `tar.gz` |
| `.deb`, `.rpm`, or Arch package | Downloads the matching package and installs it with `apt-get`, `dnf`, `zypper`, or `pacman`. This needs your password: `sudo` asks in a terminal, and a `pkexec` prompt appears when you update from the signing window |

Afterwards close the browser completely and open it again.

To find out about new releases, helperpad asks GitHub for the latest version number at most once a day, when the signing window opens. It sends nothing else.
