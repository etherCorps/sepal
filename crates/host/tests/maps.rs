//! Map, Set and WeakMap keep their semantics under QuickJS-ng's patched key
//! hash (`crates/rquickjs-sys/PATCH.md`): keys that are the same key for them
//! (1 and 1.0, 0 and -0, every NaN) are found under either form, different
//! keys stay apart, and iteration keeps insertion order through deletes and
//! the table growing. Run on both engines; V8 is the reference.

use std::path::Path;

use sepal::SvelteDocument;
use sepal::app::App;

/// Run `checks`, a script whose `check(name, ok)` calls record failures, and
/// return the names of the checks that failed.
fn failures(checks: &str) -> Vec<String> {
    let dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/pages/basic"));
    let doc = SvelteDocument::for_app(&App::from_dir(dir).unwrap()).unwrap();
    let script = format!(
        "(() => {{ const failed = []; const check = (name, ok) => {{ if (!ok) failed.push(name); }}; {checks}; return JSON.stringify(failed); }})()"
    );
    let json = doc.js().eval_string(&script).unwrap();
    serde_json::from_str(&json).unwrap()
}

/// The forms the same key takes. A `Float64Array` element is read as a
/// double, so `float(1)` is 1 stored as a float, where the literal `1` is an
/// int (arithmetic on literals is folded to an int); a NaN read from other
/// bits is another NaN.
const FORMS: &str = r#"
	const float = (n) => new Float64Array([n])[0];
	const other_nan = new Float64Array(new Uint32Array([1, 0x7ff80000]).buffer)[0];
	const same = [
		['1 and 1.0', 1, float(1)],
		['1.0 and 1', float(1), 1],
		['2^31-1 as int and as float', (2 ** 31 - 1) | 0, float(2 ** 31 - 1)],
		['-5 as int and as float', -5, float(-5)],
		['0 and -0', 0, -0],
		['-0 and 0', -0, 0],
		['0 and 0.0', 0, float(0)],
		['0 and -0.0', 0, float(-0)],
		['NaN and 0/0', NaN, 0 / 0],
		['NaN and another NaN', NaN, other_nan],
		['1n and BigInt(1)', 1n, BigInt(1)],
		['2n ** 100n, computed twice', 2n ** 100n, 2n ** 99n * 2n],
		['a string and the same, concatenated', 'abc', ['a', 'b', 'c'].join('')],
		['true and !0', true, !0]
	];
	const different = [
		['1 and 1n', 1, 1n],
		['1 and "1"', 1, '1'],
		['0 and false', 0, false],
		['undefined and null', undefined, null],
		['two objects', {}, {}],
		['two symbols', Symbol('s'), Symbol('s')],
		['NaN and undefined', NaN, undefined]
	];
"#;

#[test]
fn equal_keys_are_one_key_and_different_keys_stay_apart() {
    let failed = failures(&format!(
        r#"{FORMS}
	// A table of one key has one bucket, where any hash finds it: each map
	// starts with other keys, so that the hash picks one of thousands.
	const filler = Array.from({{ length: 5000 }}, (_, i) => 'filler ' + i);
	const map = (entries) => new Map([...filler.map((k) => [k, 0]), ...entries]);
	const set = (keys) => new Set([...filler, ...keys]);
	const F = filler.length;
	for (const [name, a, b] of same) {{
		const m = map([[a, 'a']]);
		check(`Map get: ${{name}}`, m.get(b) === 'a');
		check(`Map has: ${{name}}`, m.has(b));
		m.set(b, 'b');
		check(`Map set the other form: ${{name}}`, m.size === F + 1 && m.get(a) === 'b');
		check(`Map delete the other form: ${{name}}`, m.delete(b) && m.size === F && !m.has(a));
		const s = set([a, b]);
		check(`Set: ${{name}}`, s.size === F + 1 && s.has(a) && s.has(b));
		check(`Set delete: ${{name}}`, s.delete(b) && !s.has(a));
	}}
	for (const [name, a, b] of different) {{
		const m = map([[a, 'a'], [b, 'b']]);
		check(`Map: ${{name}}`, m.size === F + 2 && m.get(a) === 'a' && m.get(b) === 'b');
		m.delete(a);
		check(`Map delete one: ${{name}}`, !m.has(a) && m.get(b) === 'b');
		check(`Set: ${{name}}`, set([a, b]).size === F + 2);
	}}
	// -0 goes in as 0, as the spec says.
	check('-0 is kept as 0', Object.is([...new Map([[-0, 1]]).keys()][0], 0));
	check('-0 in a Set is 0', Object.is([...new Set([-0])][0], 0));
	"#
    ));
    assert!(failed.is_empty(), "{failed:#?}");
}

/// Thousands of keys of each kind, enough to grow the table many times:
/// all found, half deleted, the rest still found and in insertion order.
#[test]
fn many_keys_of_each_kind_are_found_and_kept_in_order() {
    let failed = failures(
        r#"
	const N = 10000;
	const of = (f) => Array.from({ length: N }, (_, i) => f(i));
	const kinds = {
		'ints from 1': of((i) => i + 1),
		'ints from 1M': of((i) => 1e6 + i),
		'negative ints': of((i) => -i - 1),
		'floats': of((i) => i + 0.5),
		'ints as floats': [...new Float64Array(of((i) => i + 1))],
		'objects': of(() => ({})),
		'symbols': of(() => Symbol()),
		'small BigInts': of((i) => BigInt(i)),
		'large BigInts': of((i) => 2n ** 80n + BigInt(i)),
		'strings': of((i) => 'k' + i),
		'mixed': of((i) => [i, i + 0.25, {}, 'k' + i, BigInt(i), Symbol()][i % 6])
	};
	for (const [name, keys] of Object.entries(kinds)) {
		const m = new Map();
		keys.forEach((k, i) => m.set(k, i));
		check(`${name}: size`, m.size === N);
		check(`${name}: get`, keys.every((k, i) => m.get(k) === i));
		keys.forEach((k, i) => { if (i % 2 === 0) m.delete(k); });
		check(`${name}: size after delete`, m.size === N / 2);
		check(`${name}: has after delete`, keys.every((k, i) => m.has(k) === (i % 2 === 1)));
		const order = [...m.keys()];
		check(`${name}: order after delete`, order.every((k, j) => k === keys[2 * j + 1]));
		// Re-added keys go to the end.
		m.set(keys[0], 'again');
		check(`${name}: re-added goes last`, [...m.keys()].at(-1) === keys[0] && m.size === N / 2 + 1);

		const s = new Set(keys);
		check(`${name}: Set size`, s.size === N);
		keys.forEach((k, i) => { if (i % 3 === 0) s.delete(k); });
		check(`${name}: Set has`, keys.every((k, i) => s.has(k) === (i % 3 !== 0)));
		const kept = keys.filter((_, i) => i % 3 !== 0);
		check(`${name}: Set order`, [...s].every((k, j) => k === kept[j]));
	}
	// Each int found under its float form, in a table grown many times.
	const ints = kinds['ints from 1'], floats = kinds['ints as floats'];
	const by_int = new Map(ints.map((k, i) => [k, i]));
	check('ints found as floats', floats.every((k, i) => by_int.get(k) === i));
	check('one Set of ints and floats', new Set([...ints, ...floats]).size === N);
	"#,
    );
    assert!(failed.is_empty(), "{failed:#?}");
}

#[test]
fn weak_maps_and_weak_sets_find_their_keys() {
    let failed = failures(
        r#"
	const N = 10000;
	const keys = Array.from({ length: N }, () => ({}));
	const w = new WeakMap();
	keys.forEach((k, i) => w.set(k, i));
	check('WeakMap get', keys.every((k, i) => w.get(k) === i));
	check('WeakMap has another object', !w.has({}));
	keys.forEach((k, i) => { if (i % 2 === 0) w.delete(k); });
	check('WeakMap has after delete', keys.every((k, i) => w.has(k) === (i % 2 === 1)));
	w.set(keys[0], 'again');
	check('WeakMap set again', w.get(keys[0]) === 'again');

	const ws = new WeakSet(keys);
	check('WeakSet has', keys.every((k) => ws.has(k)));
	ws.delete(keys[1]);
	check('WeakSet delete', !ws.has(keys[1]) && ws.has(keys[2]));

	// Symbols that are not in the registry can be weak keys too.
	const symbols = Array.from({ length: 1000 }, () => Symbol());
	const ws2 = new WeakMap(symbols.map((s, i) => [s, i]));
	check('WeakMap with symbol keys', symbols.every((s, i) => ws2.get(s) === i) && !ws2.has(Symbol()));
	"#,
    );
    assert!(failed.is_empty(), "{failed:#?}");
}
