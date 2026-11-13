# Releasing

Releases are built with [GoReleaser](https://goreleaser.com) from `.goreleaser.yaml`. It needs `goreleaser`, `cargo-zigbuild`, and `zig`.

```
make add-version VERSION=0.1.0   # bump Cargo.toml, commit, and tag v0.1.0
git push --follow-tags
GITHUB_TOKEN=... make release    # run the tests, build, and publish to GitHub
```

`make release` only works when the newest commit carries the version tag, and it stops if the tests fail.

`make dry-release` builds everything into `dist/` without publishing. Each release has `.deb`, `.rpm`, and Arch packages plus `tar.gz` archives, for x86_64 and arm64, and a `checksums.txt` that the installer and `helperpad update` check downloads against. The binaries link against glibc 2.31, so they run on Ubuntu 20.04 and newer, Debian 11+, Fedora, and Arch.
