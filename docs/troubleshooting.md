# Troubleshooting

Start with `helperpad status`, then `helperpad logs`.

- **Nothing in the log after clicking Sign:** the browser isn't starting helperpad. Check `helperpad status` shows it registered for that browser and the extension installed, then restart the browser fully.
- **Firefox says "extension ID mismatch in NMH manifest" or that the SDK isn't installed:** helperpad isn't registered for Firefox. Install again, or add the Firefox file from the manual install steps in the [README](../README.md#manual-install).
- **"could not open the signing window: No such file or directory":** a helper from before your last install is still running. Restart the browser.
- **"No Topaz signature pad found":** the pad isn't plugged in. Check `lsusb | grep 06a8`.
- **"Can't read the pad" or `helperpad status` says no access:** the udev rule is missing or not applied. Install again, then replug the pad.
- **"Update failed" in the signing window:** run `helperpad update` in a terminal to see the full output. For a package install the window needs a polkit agent for the password prompt; without one, the terminal is the way.
- **`helperpad update` fails with `http status: 404` right after a release:** GitHub hasn't finished publishing the files yet. Try again in a few minutes.
- **Page says the extension isn't installed:** install it for [Chrome](https://chromewebstore.google.com/detail/topaz-sigplusextlite-exte/dhcpobccjkdnmibckgpejmbpmpembgco) or [Firefox](https://addons.mozilla.org/firefox/addon/topaz-sigplusextlite-extension/) and reload the page.
- **Chromium on Ubuntu never starts helperpad:** Ubuntu's Chromium is a Snap, and its sandbox blocks helperpad. Use Google Chrome from Google's .deb.
