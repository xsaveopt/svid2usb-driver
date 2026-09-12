# svid2usb

An OBS Studio source for the StarTech SVID2USB232 composite and S-Video capture cable on macOS.

The plugin drives the cable's Empia EM28281 chip directly over USB, following the Linux em28xx driver, and hands OBS interlaced 720x480 or 720x576 frames that it deinterlaces like any other capture source.
Picture controls such as brightness, hue, sharpness, gamma and the RGB gains sit in the source's properties next to the input and video standard, and they apply live while the source is capturing.

## Installation

Unpack the zip from the latest release, then clear the quarantine flag and copy the plugin into place before restarting OBS.

```sh
xattr -dr com.apple.quarantine svid2usb.plugin
cp -R svid2usb.plugin ~/Library/Application\ Support/obs-studio/plugins/
```

## Usage

Add an SVID2USB232 Capture source, pick the input and video standard, tune the picture sliders, and set a deinterlacing mode such as Yadif 2x on it.

## Building

```sh
cargo test --workspace
cargo xtask install
```

## License

svid2usb is free software under the GNU General Public License version 2, included as LICENSE.
The way it programs the EM28281 (the register sequences, capture setup and isochronous packet format) is ported from the Linux em28xx driver in drivers/media/usb/em28xx, whose em28xx-core.c, em28xx-video.c, em28xx-cards.c and em28xx.h are GPL-2.0-or-later and whose em28xx-reg.h is GPL-2.0, so this project is a derived work released under those same terms.
That driver is the work of Ludovico Cavedon, Markus Rechberger, Mauro Carvalho Chehab, Sascha Sommer and Frank Schäfer, with em2828X support added by Bradford Love and the SVID2USB232 board added by Luciano Ciccariello.
The plugin statically links libusb, which is licensed under the LGPL 2.1.
