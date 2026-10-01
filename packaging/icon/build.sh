#!/bin/sh
# Regenerates the icon files from moqspeak.svg. Needs rsvg-convert, iconutil (macOS) and magick.
set -e
cd "$(dirname "$0")"
rm -rf moqspeak.iconset && mkdir moqspeak.iconset
for s in 16 32 128 256 512; do
  rsvg-convert -w $s -h $s moqspeak.svg -o moqspeak.iconset/icon_${s}x${s}.png
  rsvg-convert -w $((s*2)) -h $((s*2)) moqspeak.svg -o moqspeak.iconset/icon_${s}x${s}@2x.png
done
iconutil -c icns moqspeak.iconset -o moqspeak.icns
rsvg-convert -w 256 -h 256 moqspeak.svg -o moqspeak-256.png
rsvg-convert -w 512 -h 512 moqspeak.svg -o moqspeak-512.png
magick moqspeak.iconset/icon_16x16.png moqspeak.iconset/icon_32x32.png moqspeak.iconset/icon_32x32@2x.png \
  moqspeak.iconset/icon_128x128.png moqspeak-256.png moqspeak.ico
rm -rf moqspeak.iconset
