//! Modules compiled to QuickJS bytecode (`calyx --compile`, `src/bytecode.rs`):
//! they behave as they do from source, and bytecode that does not fit this
//! host, this module or this app folder is passed over for the source.
//!
//! The page is written by hand (`tests/pages/bytecode`) and copied to a
//! scratch folder for each test, which compiles it there. Whether a module
//! came from bytecode shows in `String(fn)`: compiled modules have no
//! source text, so their functions print as `[native code]`.

use std::fs;
use std::path::{Path, PathBuf};

use sepal::SvelteDocument;
use sepal::app::App;
#[cfg(not(feature = "engine-v8"))]
use sepal::bytecode::Header;
use sepal::js::compile_folder;
use serde_json::Value;

fn fixture() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/pages/bytecode"))
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// A fresh copy of `from`, unique to one test.
fn scratch_of(from: &Path, name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sepal-bytecode-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    copy_dir(from, &dir);
    dir
}

fn scratch(name: &str) -> PathBuf {
    scratch_of(fixture(), name)
}

/// Open the page in `dir` and read what its modules found.
fn run(dir: &Path, dev: bool) -> Value {
    let mut app = App::from_dir(dir).unwrap();
    app.dev = dev;
    let doc = SvelteDocument::for_app(&app).unwrap();
    let json = doc.js().eval_string("JSON.stringify(results)").unwrap();
    serde_json::from_str(&json).unwrap()
}

/// Whether `lib/math.js` was loaded from its bytecode.
fn from_bytecode(results: &Value) -> bool {
    results["source"]
        .as_str()
        .unwrap()
        .contains("[native code]")
}

#[cfg(not(feature = "engine-v8"))]
const MATH: &str = "lib/math.js.qbc";

/// Rewrite `lib/math.js.qbc`'s header.
#[cfg(not(feature = "engine-v8"))]
fn edit_header(dir: &Path, edit: impl Fn(&mut Header)) {
    let bytes = fs::read(dir.join(MATH)).unwrap();
    let (mut header, payload) = Header::decode(&bytes).unwrap();
    edit(&mut header);
    fs::write(dir.join(MATH), header.encode(payload)).unwrap();
}

#[cfg(not(feature = "engine-v8"))]
#[test]
fn a_module_from_bytecode_behaves_as_from_source() {
    let dir = scratch("same");
    let source = run(&dir, false);
    assert!(!from_bytecode(&source), "nothing is compiled yet: {source}");

    let done = compile_folder(&dir).unwrap();
    assert_eq!(done.compiled, 3, "main.js, lib/math.js, later.js");
    assert_eq!(done.left.len(), 1, "{:?}", done.left);
    assert_eq!(
        done.left[0].0, "classic.js",
        "not a module: it stays JavaScript"
    );
    let compiled = run(&dir, false);
    assert!(from_bytecode(&compiled), "{compiled}");

    // Imports, import.meta.url, a dynamic import() and a thrown error's
    // file, line and column: all as from source.
    for key in ["sum", "meta", "where", "later", "stack"] {
        assert_eq!(compiled[key], source[key], "{key}");
    }
    assert_eq!(source["sum"], 5);
    assert_eq!(source["meta"], "http://app.localhost/main.js");
    assert_eq!(source["where"], "http://app.localhost/lib/math.js");
    assert_eq!(source["later"], "from a dynamic import()");
    let stack = compiled["stack"].as_str().unwrap();
    assert!(
        stack.contains("at thrower (http://app.localhost/lib/math.js:7:"),
        "{stack}"
    );
}

#[cfg(not(feature = "engine-v8"))]
#[test]
fn bytecode_whose_header_does_not_fit_loads_the_source() {
    let cases: [(&str, fn(&mut Header)); 6] = [
        ("an older calyx", |h| h.calyx = "0.0.1".into()),
        ("another QuickJS", |h| h.engine = "QuickJS-ng 0.0.1".into()),
        ("another target", |h| {
            h.target = "riscv64gc-unknown-linux-gnu".into()
        }),
        ("another module", |h| {
            h.url = "http://app.localhost/lib/other.js".into()
        }),
        ("another size of .js", |h| h.source_len += 1),
        ("this host, unchanged", |_| {}),
    ];
    for (case, edit) in cases {
        let dir = scratch("header");
        compile_folder(&dir).unwrap();
        edit_header(&dir, edit);
        let results = run(&dir, false);
        assert_eq!(results["sum"], 5, "{case}");
        assert_eq!(
            from_bytecode(&results),
            case == "this host, unchanged",
            "{case}: {results}"
        );
    }

    let dir = scratch("magic");
    compile_folder(&dir).unwrap();
    let mut bytes = fs::read(dir.join(MATH)).unwrap();
    bytes[0] = b'X';
    fs::write(dir.join(MATH), bytes).unwrap();
    assert!(!from_bytecode(&run(&dir, false)), "not our magic");
}

/// Truncated or damaged anywhere, in the header or in QuickJS's part:
/// the source loads, and nothing crashes.
#[cfg(not(feature = "engine-v8"))]
#[test]
fn a_cut_or_damaged_qbc_loads_the_source() {
    let dir = scratch("cut");
    compile_folder(&dir).unwrap();
    let full = fs::read(dir.join(MATH)).unwrap();
    let header = full.len() - Header::decode(&full).unwrap().1.len();
    let mut damaged = full.clone();
    damaged[header + (full.len() - header) / 2] ^= 0xff;
    let cut = [
        0,
        5,
        header - 1,
        header,
        header + 1,
        header + 16,
        full.len() - 1,
    ];
    let variants = cut
        .iter()
        .map(|&len| (format!("cut to {len} bytes"), full[..len].to_vec()))
        .chain([("one byte flipped".to_owned(), damaged)]);
    for (case, bytes) in variants {
        fs::write(dir.join(MATH), bytes).unwrap();
        let results = run(&dir, false);
        assert_eq!(results["sum"], 5, "{case}");
        assert!(!from_bytecode(&results), "{case}");
    }
}

/// A `.js` rebuilt next to its old bytecode: a different size is seen on
/// every launch; the same size with other contents only in dev, where the
/// files change under a running app.
#[cfg(not(feature = "engine-v8"))]
#[test]
fn a_rebuilt_module_next_to_old_bytecode_loads_its_source() {
    let dir = scratch("rebuilt");
    compile_folder(&dir).unwrap();
    let math = fs::read_to_string(dir.join("lib/math.js")).unwrap();

    fs::write(dir.join("lib/math.js"), math.replace("a + b", "a + b + 0")).unwrap();
    assert!(!from_bytecode(&run(&dir, false)), "a longer .js");

    fs::write(dir.join("lib/math.js"), math.replace("a + b", "b + a")).unwrap();
    assert!(!from_bytecode(&run(&dir, true)), "in dev, other contents");
    assert!(
        from_bytecode(&run(&dir, false)),
        "outside dev only the size is compared"
    );
}

/// Bytecode is read only from the app folder. A symlink to a valid file
/// for this very module, outside the folder, is not followed.
#[cfg(all(unix, not(feature = "engine-v8")))]
#[test]
fn bytecode_outside_the_app_folder_is_refused() {
    let elsewhere = scratch("elsewhere");
    compile_folder(&elsewhere).unwrap();
    let app = scratch("linked");
    std::os::unix::fs::symlink(elsewhere.join(MATH), app.join(MATH)).unwrap();
    assert!(
        !from_bytecode(&run(&app, false)),
        "a symlink out of the folder"
    );

    // The same file, copied in, is taken: only where it is was wrong.
    fs::remove_file(app.join(MATH)).unwrap();
    fs::copy(elsewhere.join(MATH), app.join(MATH)).unwrap();
    assert!(from_bytecode(&run(&app, false)));
}

/// Run after every build, the compiler compiles every module; one that
/// does not compile gets no bytecode, and loses any it had.
#[cfg(not(feature = "engine-v8"))]
#[test]
fn the_compiler_compiles_every_module() {
    let dir = scratch("compiler");
    let first = compile_folder(&dir).unwrap();
    assert_eq!(first.compiled, 3);
    assert!(dir.join("lib/math.js.qbc").is_file());
    assert!(!dir.join("classic.js.qbc").exists());

    fs::write(dir.join("later.js"), "export const value = 'changed';\n").unwrap();
    assert_eq!(compile_folder(&dir).unwrap().compiled, 3);
    assert_eq!(run(&dir, false)["later"], "changed");

    fs::write(dir.join("later.js"), "export const value = ;\n").unwrap();
    let broken = compile_folder(&dir).unwrap();
    assert_eq!(broken.compiled, 2);
    assert!(broken.left.iter().any(|(module, _)| module == "later.js"));
    assert!(!dir.join("later.js.qbc").exists());
}

/// A real SvelteKit build, run from bytecode: the Tracker draws its list.
#[cfg(not(feature = "engine-v8"))]
#[test]
fn the_tracker_runs_from_bytecode() {
    let dist = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/tracker/dist"
    ));
    let dir = scratch_of(dist, "tracker");
    let done = compile_folder(&dir).unwrap();
    assert!(done.left.is_empty(), "{:?}", done.left);
    assert!(done.compiled > 30);

    let mut doc = SvelteDocument::for_app(&App::from_dir(&dir).unwrap()).unwrap();
    doc.layout(1200, 800);
    doc.wait(0.1);
    assert!(doc.query_all("[data-testid=row]").len() > 10);
    // The renderer is a module of the app like any other.
    let take = doc.js().eval_string("String(__sb.take)").unwrap();
    assert!(take.contains("[native code]"), "{take}");
}

/// V8 compiles JavaScript, and never reads what QuickJS would.
#[cfg(feature = "engine-v8")]
#[test]
fn v8_loads_the_javascript_next_to_bytecode() {
    let dir = scratch("v8");
    fs::write(dir.join("lib/math.js.qbc"), b"SEPALQBC not for V8").unwrap();
    let results = run(&dir, false);
    assert_eq!(results["sum"], 5);
    assert!(!from_bytecode(&results));
    assert!(compile_folder(&dir).unwrap().note.is_some());
}
