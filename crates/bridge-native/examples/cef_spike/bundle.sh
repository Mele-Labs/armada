#!/bin/bash
# Lay the spike out as a macOS app the way CEF needs it: the main app, the Chrome framework in
# Contents/Frameworks, and five helper apps beside it. Usage: bundle.sh <bin dir> <cef dir> <out dir>
set -euo pipefail
BIN="$1"; CEF="$2"; OUT="$3"
NAME="Armada Spike"
APP="$OUT/$NAME.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Frameworks" "$APP/Contents/Resources"

plist() { # <dir> <executable> <bundle id>
  cat > "$1/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>$2</string>
<key>CFBundleIdentifier</key><string>$3</string>
<key>CFBundleName</key><string>$2</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>0.0.0</string>
<key>CFBundleShortVersionString</key><string>0.0.0</string>
<key>LSMinimumSystemVersion</key><string>11.0</string>
<key>LSUIElement</key><string>1</string>
<key>LSEnvironment</key><dict><key>MallocNanoZone</key><string>0</string></dict>
<key>NSSupportsAutomaticGraphicsSwitching</key><true/>
</dict></plist>
EOF
}

cp "$BIN/cef_spike" "$APP/Contents/MacOS/$NAME"
plist "$APP" "$NAME" "dev.armada.cefspike"
cp -R "$CEF/Chromium Embedded Framework.framework" "$APP/Contents/Frameworks/"
for kind in "" " (GPU)" " (Renderer)" " (Plugin)" " (Alerts)"; do
  H="$NAME Helper$kind"
  D="$APP/Contents/Frameworks/$H.app"
  mkdir -p "$D/Contents/MacOS"
  cp "$BIN/cef_spike_helper" "$D/Contents/MacOS/$H"
  plist "$D" "$H" "dev.armada.cefspike.helper$(echo "$kind" | tr -dc 'A-Za-z')"
done
echo "$APP"
