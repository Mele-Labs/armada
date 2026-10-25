#!/bin/sh
#
# Install Armada from its latest GitHub Release, or move an install onto it:
#
#   curl -fsSL https://github.com/Mele-Labs/armada/releases/latest/download/install.sh | sh
#
# Puts Armada.app in /Applications (or ~/Applications when that is not
# writable), starts Fleet from the `armada` inside it under launchd, and opens
# the app. The `release` job in `.github/workflows/checks.yml` attaches this file to every release.
#
# **Downloaded with curl, so nothing is quarantined.** macOS quarantines what a
# browser downloads, and an ad hoc signed app with that flag set is refused
# with "cannot be opened". curl sets no flag, so the same app opens cleanly
# with no Developer ID. A signed download a person double-clicks is armada-67.
#
# **Plain sh, and nothing a fresh Mac lacks.** It is piped into `sh`, so no
# bash-only syntax; and no python3, whose stub on a fresh Mac opens an installer
# dialog instead of running.
#
# ARMADA_RELEASE_URL overrides where the zip comes from, for a test against a
# release that is not the latest.
set -eu

repo=Mele-Labs/armada
url="${ARMADA_RELEASE_URL:-https://github.com/$repo/releases/latest/download/Armada-macos-arm64.zip}"
label=com.armada.fleet
home="${HOME:?HOME must be set}"
support="$home/Library/Application Support/Armada"
plist="$support/$label.plist"
domain="gui/$(id -u)"

say() { printf '\033[1m%s\033[0m\n' "$*"; }
fail() {
  echo "install: $*" >&2
  exit 1
}

[ "$(uname -s)" = Darwin ] || fail "Armada runs on macOS only"
[ "$(uname -m)" = arm64 ] || fail "Armada is built for Apple Silicon only, and this Mac is $(uname -m)"

# Fleet works in git repositories, and git on a fresh Mac is the Command Line
# Tools. Asking for them opens Apple's own installer, which needs a person.
if ! xcode-select -p >/dev/null 2>&1; then
  xcode-select --install >/dev/null 2>&1 || true
  fail "Armada needs Apple's Command Line Tools for git. Finish the installer that just opened, then run this again"
fi

# **A Fleet run from a checkout is a developer's, and this never replaces it.**
# `scripts/restart` writes the same label pointing at `~/.cargo/bin/armada`; an
# install pointing there would take over a Fleet somebody is building.
if [ -f "$plist" ]; then
  current=$(/usr/libexec/PlistBuddy -c "Print :ProgramArguments:0" "$plist" 2>/dev/null || true)
  case "$current" in
    */Armada.app/Contents/Resources/bin/armada) ;;
    *) fail "Fleet here runs $current, not an installed Armada. Run scripts/restart in that checkout to update it" ;;
  esac
fi

if [ -w /Applications ]; then apps=/Applications; else apps="$home/Applications"; fi
mkdir -p "$apps"
app="$apps/Armada.app"
armada="$app/Contents/Resources/bin/armada"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

say "Downloading Armada"
curl -fL --progress-bar -o "$tmp/Armada.zip" "$url" || fail "could not download $url"
ditto -x -k "$tmp/Armada.zip" "$tmp" || fail "the download is not a readable zip"
[ -x "$tmp/Armada.app/Contents/Resources/bin/armada" ] || fail "the download holds no Fleet: $url"
read -r version _ < "$tmp/Armada.app/Contents/Resources/version" || version=unknown

# The app is closed before it is replaced. Fleet stops below, and a working
# Drone is not adopted the way an update from inside the app adopts it.
osascript -e 'quit app "Armada"' >/dev/null 2>&1 || true

say "Installing Armada $version into $apps"
rm -rf "$app"
mv "$tmp/Armada.app" "$app"

# The same job `scripts/restart` writes, pointed at the bundled binary:
# docs/concepts/fleet.md, Daemon lifecycle, says why each key is what it is.
# Fleet starts with no repository and serves what it is given from Bridge.
xml() { printf '%s' "$1" | sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g'; }
mkdir -p "$support" "$home/Library/Logs/Armada"
log="$home/Library/Logs/Armada/$label.log"
cat > "$plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>$label</string>
  <key>ProgramArguments</key>
  <array><string>$(xml "$armada")</string><string>serve</string></array>
  <key>KeepAlive</key><dict><key>SuccessfulExit</key><false/></dict>
  <key>ThrottleInterval</key><integer>2</integer>
  <key>RunAtLoad</key><false/>
  <key>ProcessType</key><string>Interactive</string>
  <key>StandardOutPath</key><string>$(xml "$log")</string>
  <key>StandardErrorPath</key><string>$(xml "$log")</string>
  <key>EnvironmentVariables</key>
  <dict>
    <key>HOME</key><string>$(xml "$home")</string>
    <key>PATH</key><string>$(xml "$PATH")</string>
  </dict>
</dict>
</plist>
EOF
chmod 644 "$plist"
plutil -lint "$plist" >/dev/null || fail "wrote a plist launchd cannot read: $plist"

# **Out and back in, never kickstart**: `kickstart -k` restarts the definition
# launchd already holds and ignores the plist just written (`scripts/restart`).
say "Starting Fleet"
launchctl bootout "$domain/$label" >/dev/null 2>&1 || true
i=0
while launchctl print "$domain/$label" >/dev/null 2>&1; do
  i=$((i + 1))
  [ "$i" -le 30 ] || fail "the old Fleet has not stopped after 30 seconds; run this again once it has"
  sleep 1
done
launchctl bootstrap "$domain" "$plist" || fail "launchd would not start Fleet from $plist; its log is $log"

open "$app"
say "Armada $version is installed"
