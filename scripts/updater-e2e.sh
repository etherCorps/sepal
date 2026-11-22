#!/bin/sh
# The updater, for real, on macOS: an installed 0.1.0 of examples/updater finds
# 0.2.0 on a local server, downloads it, verifies it, replaces itself and
# relaunches. Opens two windows briefly. Needs `pnpm run build:host` first.
#
#   sh scripts/updater-e2e.sh
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
host="$root/packages/host/bin/darwin-arm64/svelte-blitz-host"
[ -x "$host" ] || { echo "no host binary; run: pnpm run build:host" >&2; exit 1; }

work=$(mktemp -d /tmp/svelte-blitz-updater-XXXXXX)
port=8617
app="$work/installed/Updater Demo.app"
server_pid=""
cleanup() {
	[ -n "$server_pid" ] && kill "$server_pid" 2>/dev/null || true
	pkill -f "$work/installed/Updater Demo.app/Contents/MacOS/Updater Demo" 2>/dev/null || true
	rm -rf "$work"
}
trap cleanup EXIT

# The developer's key pair. The public half is built into the app.
pubkey=$("$host" --updater-keygen "$work/update.key" 2>/dev/null)

build() { # version
	( cd "$root/examples/updater" &&
		APP_VERSION=$1 UPDATE_URL="http://127.0.0.1:$port/latest.json" UPDATE_PUBKEY="$pubkey" \
			VITE_AUTO_UPDATE=1 pnpm exec vite build --logLevel warn )
}

# What the user has installed: 0.1.0.
build 0.1.0
"$host" --bundle "$root/examples/updater/dist" "$work/installed" >/dev/null 2>&1

# The release: 0.2.0 as a signed update, published on a local server.
build 0.2.0
"$host" --bundle "$root/examples/updater/dist" "$work/release" --update-key "$work/update.key" \
	--update-notes "Second version" >/dev/null 2>&1
( cd "$work/release" && exec python3 -m http.server "$port" --bind 127.0.0.1 >/dev/null 2>&1 ) &
server_pid=$!
sleep 1

version() { /usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$app/Contents/Info.plist"; }
echo "installed: $(version)"

# Start the installed app the way Finder does. It is built to update at launch.
"$app/Contents/MacOS/Updater Demo" >"$work/first.log" 2>&1 &
first=$!

i=0
while [ "$(version)" != "0.2.0" ]; do
	i=$((i + 1))
	[ "$i" -gt 60 ] && { echo "FAILED: still $(version) after 30 s" >&2; cat "$work/first.log" >&2; exit 1; }
	sleep 0.5
done
echo "on disk:   $(version)"

# The old process exits and a new one starts from the same place.
i=0
while kill -0 "$first" 2>/dev/null; do
	i=$((i + 1))
	[ "$i" -gt 20 ] && { echo "FAILED: the old version is still running" >&2; exit 1; }
	sleep 0.5
done
sleep 2
new=$(pgrep -f "$app/Contents/MacOS/Updater Demo" | head -1)
[ -n "$new" ] && [ "$new" != "$first" ] || { echo "FAILED: the new version was not relaunched" >&2; exit 1; }
echo "running:   pid $new (was $first), $(lsappinfo info -only name -app "$new" | head -1)"

codesign --verify --deep --strict "$app" && echo "signature: valid after the update"
echo "ok"
