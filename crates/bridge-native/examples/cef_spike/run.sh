#!/bin/bash
# Build, bundle and run the spike with no window on screen, drive it over CDP, and sample the
# Chrome processes while it paints. Needs cmake and ninja on PATH for the cef crate's first build.
# Usage: run.sh <scratch dir>   (output pictures and logs land in <scratch dir>/out)
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
S="${1:?scratch dir}"
mkdir -p "$S/out"
export CEF_PATH="${CEF_PATH:-$S/cef}" CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$S/target}"
# Start from bridge-native's lockfile so gpui and its dependencies resolve to the versions Bridge ships.
[ -f "$HERE/Cargo.lock" ] || cp "$HERE/../../Cargo.lock" "$HERE/Cargo.lock"
(cd "$HERE" && cargo build)
CEFDIR="$(find "$CEF_PATH" -maxdepth 2 -name 'cef_macos_aarch64' | head -1)"
"$HERE/bundle.sh" "$CARGO_TARGET_DIR/debug" "$CEFDIR" "$S/app" >/dev/null
export CEF_SPIKE_PAGE="$HERE/page.html${PAGE_QUERY:-}" CEF_SPIKE_OUT="$S/out" CEF_SPIKE_SECS="${CEF_SPIKE_SECS:-14}"
"$S/app/Armada Spike.app/Contents/MacOS/Armada Spike" > "$S/out/run.log" 2>&1 &
APP=$!
node "$HERE/cdp.mjs" "${CEF_SPIKE_PORT:-9333}" "$S/out" > "$S/out/cdp.log" 2>&1 &
CDP=$!
sleep 3.5
# CPU seconds of every process Chrome started over 1.5 s of quiet, and their resident memory.
snap() { ps -axo pid=,time=,rss=,command= | awk '/Armada Spike/ && !/awk/ {split($2,t,":"); c=substr($0, index($0,$4)); sub(/ --.*/, "", c); gsub(/.*Frameworks\//, "", c); gsub(/.*MacOS\//, "", c); gsub(" ", "_", c); printf "%s %.2f %s %s\n", $1, t[1]*60+t[2], $3, c}'; }
snap > "$S/out/ps-a.txt"; sleep 1.5; snap > "$S/out/ps-b.txt"
join -j1 <(sort "$S/out/ps-a.txt") <(sort "$S/out/ps-b.txt") | awk '{printf "%s cpu %.0f%% rss %d MB  %s\n", $1, ($5-$2)/1.5*100, $6/1024, $7}' > "$S/out/ps.txt" || true
wait "$APP"; wait "$CDP" || true
cat "$S/out/run.log" "$S/out/cdp.log"
