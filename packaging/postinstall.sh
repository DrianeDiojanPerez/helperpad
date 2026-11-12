#!/bin/sh
# Applies the pad's udev rule without a reboot. A pad that's already plugged in
# still needs to be replugged.
if command -v udevadm >/dev/null 2>&1; then
    udevadm control --reload || true
    udevadm trigger --attr-match=idVendor=06a8 || true
fi
echo "helperpad installed. Replug the Topaz pad, add the Topaz SigPlusExtLite extension in Chrome, and restart Chrome."
