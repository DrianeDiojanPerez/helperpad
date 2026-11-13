# How it works

helperpad is a Linux stand-in for Topaz's Windows-only SigPlusExtLite helper. With it, websites that use the Topaz SigPlusExtLite browser extension can capture signatures from a Topaz HSB pad on Linux.

```
Website → SigPlusExtLite extension → helperpad → Topaz pad (/dev/hidrawN)
```

Chrome and Firefox start `helperpad` as the native messaging host `com.topaz.sigplusextlite.win.2`. When a page asks for a signature, helperpad opens a signing window with live ink and Clear / OK / Cancel buttons, then sends the signature back to the page.

The pad needs no driver. The Linux kernel's built-in HID driver already runs it, but by default only root can read it, so the install adds a udev rule that gives your user access.

Built for the Topaz T-L460-HSB-R (USB ID `06a8:0043`).
