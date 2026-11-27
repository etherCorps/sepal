# Patched copy of rquickjs-sys 0.14.0

This is the crates.io source of `rquickjs-sys` 0.14.0 (MIT,
https://github.com/DelSkayn/rquickjs), which carries QuickJS-ng 0.16.2 (MIT,
https://github.com/quickjs-ng/quickjs) in `quickjs/`, with one change, in
`quickjs/quickjs.c`. `crates/host/Cargo.toml` swaps it in through
`[patch.crates-io]`. The change is marked `sepal patch`.

**Drop this copy** once rquickjs ships a QuickJS-ng release that has
quickjs-ng PR #1931 (see below): remove the `[patch.crates-io]` line and this
folder together, and bump rquickjs as its own commit.

## The change

**1. Map and Set keys spread over the buckets.** `quickjs/quickjs.c`,
`map_hash_key`: the hash of every key that is not a string goes through
murmur3's 32-bit finalizer before the table takes its low bits as the bucket.
The context's seed is mixed in before the finalizer, so it changes buckets
too. Strings hash as before.

Upstream, a number key is hashed as `(low word ^ high word) * 3163` of its
double, an object or symbol as its address times 3163, a small BigInt as the
double of its value, and the bucket is the low bits of that. A small integer's
double ends in zero bits, and an object's address is aligned, so those low
bits hardly change from key to key: node ids 1 to 10,000 fell in 64 of 8,192
buckets, and every lookup walked a list. The renderer keys its node maps by id
(`elements`, `by_tag` in `renderer.js`), so every first visit to a screen paid
for it.

Keys that are the same for Map and Set still hash the same: the finalizer is a
function of the same value. 1 and 1.0 both hash as the double 1, -0 has
already become 0 (`map_normalize_key`), and every NaN is the one NaN.
`crates/host/tests/maps.rs` checks those, and order, deletes and growth for
each kind of key, on both engines.

Measured with `drive`'s `eval:` (release, `--features screenshot`): one round
is a new Map that sets, gets, tests and deletes 10,000 keys, then a new Set
that adds and tests them. Median of three launches, ms a round:

| keys | before | after |
|---|---:|---:|
| ints 1 to 10,000 | 91.5 | 3.35 |
| small BigInts | 94.2 | 3.30 |
| floats (i + 0.5) | 19.9 | 3.45 |
| objects | 21.8 | 3.25 |
| symbols | 14.1 | 3.25 |
| ints from 1,000,000 | 2.55 | 3.40 |
| strings (unchanged) | 3.10 | 3.20 |

Ints from a million were spread well by luck: their doubles' high words
change with every key. They now cost what strings do. The Tracker's Board,
headless at 1,000 issues, five launches: the first visit went from 137-147 ms
to 113-123, a repeat visit from 79 to 73-76.

**Upstream.** Reported in quickjs-ng issue #1815, and fixed in PR #1931
("Hash table fixes", merged 22 Nov 2026), which no release has yet (0.18.0
still multiplies by 3163). Upstream hashes every non-string key with a keyed
hash (wyhash on 64-bit targets), removes the old seed, stores each record's
hash, and hardens the shape, property and object tables against crafted
collisions. This change is smaller and only fixes the spread: murmur3's
finalizer is not keyed, so keys chosen to collide still can.

## Editing it

`build.rs` copies `quickjs/` to `OUT_DIR` and compiles it there, and only
reruns when an environment variable changes. After editing a C file, `touch
build.rs`, or the old object is linked.
