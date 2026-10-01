#!/bin/sh
# Builds dist/moqspeak.app from a release binary.
#   packaging/macos/bundle.sh [path/to/moqspeak]
# Signs ad hoc unless MACOS_SIGNING_IDENTITY names a Developer ID certificate in the keychain.
set -e
cd "$(dirname "$0")/../.."
BIN=${1:-target/release/moqspeak}
APP=dist/moqspeak.app
VERSION=$(grep -m1 '^version' client/Cargo.toml | cut -d'"' -f2)
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/moqspeak"
cp packaging/icon/moqspeak.icns "$APP/Contents/Resources/moqspeak.icns"
sed "s/__VERSION__/$VERSION/g" packaging/macos/Info.plist > "$APP/Contents/Info.plist"
if [ -n "$MACOS_SIGNING_IDENTITY" ]; then
  codesign --force --options runtime --timestamp \
    --entitlements packaging/macos/entitlements.plist \
    --sign "$MACOS_SIGNING_IDENTITY" "$APP"
else
  codesign --force --options runtime --entitlements packaging/macos/entitlements.plist --sign - "$APP"
fi
codesign --verify --strict "$APP"
echo "built $APP ($VERSION)"
