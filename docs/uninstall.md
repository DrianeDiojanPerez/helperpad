# Uninstall

```
helperpad uninstall
```

It asks before removing anything (`--yes` skips the question) and works out how helperpad was installed:

- **From a package:** removes the package with `apt-get`, `dnf`, `zypper`, or `pacman`. This needs your password.
- **From the installer or from source:** removes the binary and the Chrome and Firefox registration in your home folder. The pad's udev rule stays; the command prints the `sudo rm` line to remove it too.

The log stays in `~/.local/state/helperpad/`. Delete that folder to remove it too.
