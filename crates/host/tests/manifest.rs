//! The app manifest (`app.json`) and its icon, without a window.
//!
//! Build the bundle first: `pnpm run build` in the repository root.

use std::path::{Path, PathBuf};

use sepal::app::{App, Manifest, decode_icon};

fn counter_dist() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/counter/dist"
    ))
}

/// An empty scratch directory, unique to one test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sepal-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn reads_what_the_vite_preset_wrote() {
    let app = App::from_dir(counter_dist()).expect("run `pnpm run build` first");
    assert!(!app.js.is_empty() && !app.css.is_empty());

    let manifest = app.manifest;
    assert_eq!(manifest.name.as_deref(), Some("Counter"));
    assert_eq!(manifest.title.as_deref(), Some("Svelte on Blitz"));
    assert_eq!(manifest.size(), (800, 600));

    let icon = manifest.icon.expect("the example declares an icon");
    assert_eq!((icon.width, icon.height), (1024, 1024));
    assert_eq!(icon.rgba.len(), 1024 * 1024 * 4);
    // The corner is outside the rounded tile, the centre is the white bolt.
    assert_eq!(icon.rgba[3], 0, "corner is transparent");
    let centre = (512 * 1024 + 512) * 4;
    assert_eq!(&icon.rgba[centre..centre + 4], &[255, 255, 255, 255]);
}

#[test]
fn a_directory_without_a_manifest_gets_defaults() {
    let dir = scratch("no-manifest");
    std::fs::write(dir.join("app.js"), "").unwrap();

    let app = App::from_dir(&dir).unwrap();
    assert_eq!(app.css, "", "app.css is optional");
    assert!(app.manifest.title.is_none() && app.manifest.icon.is_none());
    assert_eq!(app.manifest.size(), (800, 600));

    let missing = App::from_dir(&scratch("empty")).err().unwrap();
    assert!(missing.contains("app.js"), "{missing}");
}

#[test]
fn manifest_fields_are_optional_and_lenient() {
    let dir = Path::new(".");
    let partial = Manifest::parse(r#"{ "window": { "width": 1024 } }"#, dir).unwrap();
    assert_eq!(partial.size(), (1024, 600));
    assert!(partial.name.is_none() && partial.title.is_none());

    // Wrong types and nonsense sizes fall back instead of failing the launch.
    let odd = Manifest::parse(
        r#"{ "name": 7, "window": { "width": -5, "height": 0 } }"#,
        dir,
    );
    assert_eq!(odd.unwrap().size(), (800, 600));
    assert_eq!(Manifest::parse("[]", dir).unwrap().size(), (800, 600));

    let broken = Manifest::parse("{ not json", dir).err().unwrap();
    assert!(broken.starts_with("app.json:"), "{broken}");
}

#[test]
fn a_bad_icon_is_an_error_not_a_panic() {
    let not_png = decode_icon(b"definitely not a png".to_vec()).err().unwrap();
    assert!(not_png.contains("not a valid PNG"), "{not_png}");

    // A real header followed by garbage.
    let mut truncated = std::fs::read(counter_dist().join("icon.png")).unwrap();
    truncated.truncate(200);
    assert!(decode_icon(truncated).is_err());

    let dir = scratch("bad-icon");
    std::fs::write(dir.join("icon.png"), b"\x89PNG\r\n\x1a\nnope").unwrap();
    let error = Manifest::parse(r#"{ "icon": "icon.png" }"#, &dir)
        .err()
        .unwrap();
    assert!(
        error.contains("icon.png") && error.contains("not a valid PNG"),
        "{error}"
    );

    let missing = Manifest::parse(r#"{ "icon": "nope.png" }"#, &dir)
        .err()
        .unwrap();
    assert!(missing.contains("cannot read"), "{missing}");
}

#[test]
fn the_icon_cannot_point_outside_the_app_directory() {
    let dir = scratch("escape");
    for path in ["../icon.png", "/etc/hosts", "a/../../icon.png"] {
        let json = format!(r#"{{ "icon": "{path}" }}"#);
        let error = Manifest::parse(&json, &dir).err().unwrap();
        assert!(
            error.contains("inside the app directory"),
            "{path}: {error}"
        );
    }
}
