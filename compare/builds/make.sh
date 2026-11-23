#!/bin/sh
# svelte-blitz builds of the bench Tracker (compare/porting-blitz.md). Run from
# the repository root after the release hosts (see porting-blitz.md).
# The app is a bench build (VITE_BENCH=1: no seed, marks from the first line),
# into its own outDir so examples/tracker/dist stays the normal build.
# The -diag apps run hosts built with blitz-dom's log-phase-times (per-phase
# resolve timings on stdout): diagnostics only, never measured. Own app ids.
set -e
VITE_BENCH=1 pnpm --filter tracker exec vite build --outDir ../../compare/builds/dist-bench --emptyOutDir >/dev/null
cd compare/builds
for column in blitz:quickjs:blitz blitzv8:v8:blitzv8 blitzdiag:quickjs-diag:blitz-diag blitzv8diag:v8-diag:blitzv8-diag; do
	name=${column%%:*} rest=${column#*:}
	engine=${rest%%:*} app=${rest#*:}
	[ -x hosts/svelte-blitz-host-$engine ] || { echo "skipping $name: no hosts/svelte-blitz-host-$engine"; continue; }
	rm -rf dist-$name out-$name Tracker-$app.app
	cp -R dist-bench dist-$name
	node -e "const f='dist-$name/app.json',m=JSON.parse(require('fs').readFileSync(f));m.id='com.compare.tracker.$name';require('fs').writeFileSync(f,JSON.stringify(m,null,'\t'))"
	hosts/svelte-blitz-host-$engine --bundle dist-$name out-$name
	mv out-$name/Tracker.app Tracker-$app.app
	rm -rf out-$name dist-$name
done
rm -rf dist-bench
