//! The updater: keys and signatures, the feed's rules, and whole updates
//! against a server on this machine. No outside network is used.
//!
//! Build the bundle first: `pnpm run build` in the repository root.

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sepal::SvelteDocument;
use sepal::app::{App, Manifest};
use sepal::updater::{
    self, Config, Updater, generate_keypair, parse_feed, platform, sign, verify, write_update,
};

/// An empty scratch directory, unique to one test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sepal-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A stand-in app bundle whose contents say which version it is.
fn fake_bundle(dir: &Path, version: &str) -> PathBuf {
    let bundle = dir.join("Demo.app");
    fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
    fs::create_dir_all(bundle.join("Contents/Resources")).unwrap();
    fs::write(
        bundle.join("Contents/MacOS/Demo"),
        format!("binary {version}"),
    )
    .unwrap();
    fs::write(
        bundle.join("Contents/Resources/index.html"),
        format!("// {version}"),
    )
    .unwrap();
    bundle
}

fn installed_version(bundle: &Path) -> String {
    fs::read_to_string(bundle.join("Contents/MacOS/Demo")).unwrap()
}

/// Serves a set of files over HTTP on this machine. Files can be swapped while
/// it runs, which is how the tests play a tampering server.
struct Server {
    base: String,
    files: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl Server {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        let files: Arc<Mutex<HashMap<String, Vec<u8>>>> = Arc::default();
        let served = files.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut request = String::new();
                if BufReader::new(&stream).read_line(&mut request).is_err() {
                    continue;
                }
                let path = request.split(' ').nth(1).unwrap_or("/").to_owned();
                let body = served.lock().unwrap().get(&path).cloned();
                let (status, body) = match body {
                    Some(body) => ("200 OK", body),
                    None => ("404 Not Found", Vec::new()),
                };
                let head = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&body);
            }
        });
        Self { base, files }
    }

    fn put(&self, path: &str, body: impl Into<Vec<u8>>) {
        self.files
            .lock()
            .unwrap()
            .insert(path.to_owned(), body.into());
    }

    /// Publish everything in a directory, as an upload of a release would.
    fn publish(&self, dir: &Path) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                let name = entry.file_name().into_string().unwrap();
                self.put(&format!("/{name}"), fs::read(entry.path()).unwrap());
            }
        }
    }

    fn feed_url(&self) -> String {
        format!("{}/latest.json", self.base)
    }
}

#[test]
fn signatures() {
    let (private, public) = generate_keypair().unwrap();
    assert_eq!(updater::public_key(&private).unwrap(), public);
    let data = b"the update archive";
    let signature = sign(&private, data).unwrap();

    verify(&public, data, &signature).unwrap();

    // One changed byte, a signature from another key, and plain garbage all fail.
    let changed = verify(&public, b"the update archivE", &signature)
        .err()
        .unwrap();
    assert!(changed.contains("does not match"), "{changed}");
    let (other_private, other_public) = generate_keypair().unwrap();
    assert!(verify(&other_public, data, &signature).is_err());
    assert!(verify(&public, data, &sign(&other_private, data).unwrap()).is_err());
    assert!(verify(&public, data, "not base64!").is_err());
    assert!(verify("AAAA", data, &signature).is_err());
    assert!(sign("too short", data).is_err());

    // Two generated keys are never the same.
    assert_ne!(private, other_private);
}

#[test]
fn feed_rules() {
    let feed_url = "https://example.com/updates/latest.json";
    let feed = |version: &str, url: &str| {
        format!(
            r#"{{ "version": "{version}", "notes": "Faster", "pub_date": "2026-11-22T12:00:00Z",
                 "platforms": {{ "darwin-arm64": {{ "url": "{url}", "signature": "c2ln" }} }} }}"#
        )
    };
    let check =
        |json: &str, current: &str, platform: &str| parse_feed(json, feed_url, current, platform);

    let update = check(&feed("0.2.0", "Demo.tar.gz"), "0.1.0", "darwin-arm64")
        .unwrap()
        .expect("0.2.0 is newer than 0.1.0");
    assert_eq!(update.version, "0.2.0");
    assert_eq!(update.notes.as_deref(), Some("Faster"));
    assert_eq!(update.date.as_deref(), Some("2026-11-22T12:00:00Z"));
    // A relative URL sits next to the feed.
    assert_eq!(update.url, "https://example.com/updates/Demo.tar.gz");
    assert_eq!(update.signature, "c2ln");

    // The same or an older version is never an update, whatever the feed offers.
    assert!(
        check(&feed("0.1.0", "x"), "0.1.0", "darwin-arm64")
            .unwrap()
            .is_none()
    );
    assert!(
        check(&feed("0.1.0", "x"), "0.2.0", "darwin-arm64")
            .unwrap()
            .is_none()
    );
    // Versions compare as versions, not as text.
    assert!(
        check(&feed("0.10.0", "x"), "0.9.0", "darwin-arm64")
            .unwrap()
            .is_some()
    );
    assert!(
        check(&feed("v1.0.0", "x"), "1.0.0-beta.1", "darwin-arm64")
            .unwrap()
            .is_some()
    );
    // Nothing for this platform.
    assert!(
        check(&feed("0.2.0", "x"), "0.1.0", "linux-x64-gnu")
            .unwrap()
            .is_none()
    );

    // An archive may live elsewhere, but only over https (or on this machine).
    let elsewhere = check(
        &feed("0.2.0", "https://cdn.example.com/a.tar.gz"),
        "0.1.0",
        "darwin-arm64",
    );
    assert_eq!(
        elsewhere.unwrap().unwrap().url,
        "https://cdn.example.com/a.tar.gz"
    );
    let insecure = check(
        &feed("0.2.0", "http://cdn.example.com/a.tar.gz"),
        "0.1.0",
        "darwin-arm64",
    );
    assert!(insecure.err().unwrap().contains("not an https URL"));
    assert!(check(&feed("0.2.0", "file:///etc/hosts"), "0.1.0", "darwin-arm64").is_err());

    assert!(check("not json", "0.1.0", "darwin-arm64").is_err());
    assert!(check(r#"{ "platforms": {} }"#, "0.1.0", "darwin-arm64").is_err());
    assert!(check(&feed("soon", "x"), "0.1.0", "darwin-arm64").is_err());
    let incomplete = r#"{ "version": "0.2.0", "platforms": { "darwin-arm64": { "url": "x" } } }"#;
    assert!(check(incomplete, "0.1.0", "darwin-arm64").is_err());
}

#[test]
fn the_feed_is_only_fetched_over_https() {
    let (_, public) = generate_keypair().unwrap();
    let config = Config {
        url: "http://example.com/latest.json".into(),
        pubkey: public,
    };
    let error = Updater::new(config, "0.1.0", None).check().err().unwrap();
    assert!(error.contains("not an https URL"), "{error}");
}

#[test]
fn building_an_update() {
    let dir = scratch("build-update");
    let bundle = fake_bundle(&dir, "0.2.0");
    let out = dir.join("release");
    let (private, public) = generate_keypair().unwrap();

    let archive = write_update(
        &bundle,
        &out,
        "0.2.0",
        Some("Faster"),
        &private,
        Some(&public),
    )
    .unwrap();
    let name = format!("Demo-0.2.0-{}.app.tar.gz", platform());
    assert_eq!(archive, out.join(&name));

    // The signature file and the feed both carry the archive's signature.
    let signature = fs::read_to_string(out.join(format!("{name}.sig"))).unwrap();
    verify(&public, &fs::read(&archive).unwrap(), &signature).unwrap();
    let feed = fs::read_to_string(out.join("latest.json")).unwrap();
    let update = parse_feed(
        &feed,
        "https://example.com/latest.json",
        "0.1.0",
        &platform(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(update.signature, signature);
    assert_eq!(update.url, format!("https://example.com/{name}"));
    assert_eq!(update.notes.as_deref(), Some("Faster"));

    // Another platform's build of the same version joins the feed instead of replacing it.
    let mut other: serde_json::Value = serde_json::from_str(&feed).unwrap();
    other["platforms"]["some-other-os"] = serde_json::json!({ "url": "x", "signature": "y" });
    other["platforms"]
        .as_object_mut()
        .unwrap()
        .remove(&platform());
    fs::write(out.join("latest.json"), other.to_string()).unwrap();
    write_update(&bundle, &out, "0.2.0", None, &private, Some(&public)).unwrap();
    let merged: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out.join("latest.json")).unwrap()).unwrap();
    assert!(merged["platforms"]["some-other-os"].is_object());
    assert!(merged["platforms"][platform()].is_object());
    // A new version starts a new feed.
    write_update(&bundle, &out, "0.3.0", None, &private, Some(&public)).unwrap();
    let fresh: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out.join("latest.json")).unwrap()).unwrap();
    assert_eq!(fresh["version"], "0.3.0");
    assert!(fresh["platforms"]["some-other-os"].is_null());

    // Signing with a key the installed apps do not trust is refused up front.
    let (stranger, _) = generate_keypair().unwrap();
    let mismatch = write_update(&bundle, &out, "0.4.0", None, &stranger, Some(&public));
    assert!(
        mismatch
            .err()
            .unwrap()
            .contains("does not match `updater.pubkey`")
    );
    assert!(write_update(&bundle, &out, "next", None, &private, None).is_err());
}

/// The real thing, end to end, minus the relaunch: an installed 0.1.0 finds
/// 0.2.0 on a server, downloads it, verifies it and replaces itself.
#[cfg(target_os = "macos")]
#[test]
fn an_app_updates_itself() {
    let dir = scratch("self-update");
    let (private, public) = generate_keypair().unwrap();
    let server = Server::start();

    // The release: build 0.2.0 somewhere else and publish its three files.
    let release = dir.join("release");
    let new_build = fake_bundle(&dir.join("build"), "0.2.0");
    fs::create_dir_all(&release).unwrap();
    write_update(
        &new_build,
        &release,
        "0.2.0",
        Some("Faster"),
        &private,
        Some(&public),
    )
    .unwrap();
    server.publish(&release);

    // The user's machine: 0.1.0, installed under a name of the user's choosing.
    let installed = fake_bundle(&dir.join("Applications"), "0.1.0");
    let renamed = installed.with_file_name("My Demo.app");
    fs::rename(&installed, &renamed).unwrap();
    fs::write(renamed.join("Contents/Resources/stale.txt"), "from 0.1.0").unwrap();

    let config = Config {
        url: server.feed_url(),
        pubkey: public,
    };
    let updater = Updater::new(config, "0.1.0", Some(renamed.clone()));

    let update = updater.check().unwrap().expect("0.2.0 is on the server");
    assert_eq!(
        (update.version.as_str(), update.notes.as_deref()),
        ("0.2.0", Some("Faster"))
    );
    updater.download().unwrap();
    assert_eq!(
        installed_version(&renamed),
        "binary 0.1.0",
        "downloading changes nothing"
    );

    assert_eq!(updater.install().unwrap(), renamed);
    assert_eq!(installed_version(&renamed), "binary 0.2.0");
    assert!(
        !renamed.join("Contents/Resources/stale.txt").exists(),
        "the app was replaced, not merged"
    );
    let left_over: Vec<_> = fs::read_dir(renamed.parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(
        left_over,
        ["My Demo.app"],
        "no scratch directories left behind"
    );
}

#[test]
fn a_tampered_or_foreign_update_is_never_installed() {
    let dir = scratch("tampered");
    let (private, public) = generate_keypair().unwrap();
    let server = Server::start();
    let release = dir.join("release");
    let new_build = fake_bundle(&dir.join("build"), "0.2.0");
    fs::create_dir_all(&release).unwrap();
    let archive =
        write_update(&new_build, &release, "0.2.0", None, &private, Some(&public)).unwrap();
    server.publish(&release);
    let archive_path = format!("/{}", archive.file_name().unwrap().to_str().unwrap());

    let installed = fake_bundle(&dir.join("Applications"), "0.1.0");
    let updater = || {
        let config = Config {
            url: server.feed_url(),
            pubkey: public.clone(),
        };
        Updater::new(config, "0.1.0", Some(installed.clone()))
    };

    // The archive on the server is changed after it was signed.
    let mut bytes = fs::read(&archive).unwrap();
    let middle = bytes.len() / 2;
    bytes[middle] ^= 0xff;
    server.put(&archive_path, bytes);
    let tampered = updater();
    assert!(tampered.check().unwrap().is_some());
    let error = tampered.install().err().unwrap();
    assert!(error.contains("signature does not match"), "{error}");
    assert_eq!(installed_version(&installed), "binary 0.1.0");

    // Someone else publishes a properly signed update, but with their own key.
    let (stranger, _) = generate_keypair().unwrap();
    let theirs = dir.join("theirs");
    let evil_build = fake_bundle(&dir.join("evil"), "9.9.9");
    write_update(&evil_build, &theirs, "9.9.9", None, &stranger, None).unwrap();
    server.publish(&theirs);
    let foreign = updater();
    assert_eq!(foreign.check().unwrap().unwrap().version, "9.9.9");
    assert!(
        foreign
            .install()
            .err()
            .unwrap()
            .contains("signature does not match")
    );
    assert_eq!(installed_version(&installed), "binary 0.1.0");

    // The archive is gone from the server.
    server
        .files
        .lock()
        .unwrap()
        .retain(|path, _| path == "/latest.json");
    let missing = updater();
    missing.check().unwrap();
    assert!(missing.download().err().unwrap().contains("cannot fetch"));

    // Asking to download or install before looking is an error, not a crash.
    assert!(
        updater()
            .download()
            .err()
            .unwrap()
            .contains("call check() first")
    );
}

#[test]
fn the_manifest_carries_the_updater_settings() {
    let dir = Path::new(".");
    let manifest = Manifest::parse(
        r#"{ "version": "1.2.3", "updater": { "url": "https://example.com/latest.json", "pubkey": "abc" } }"#,
        dir,
    )
    .unwrap();
    let expected = Config {
        url: "https://example.com/latest.json".into(),
        pubkey: "abc".into(),
    };
    assert_eq!(manifest.updater, Some(expected));
    assert!(Manifest::parse("{}", dir).unwrap().updater.is_none());

    // Half a configuration is an error, not an app that silently never updates.
    let half = Manifest::parse(
        r#"{ "updater": { "url": "https://example.com/latest.json" } }"#,
        dir,
    );
    assert!(
        half.err()
            .unwrap()
            .contains("needs both `url` and `pubkey`")
    );
}

/// Wait for a host call to come back and the page to show it.
#[track_caller]
fn wait_for_text(doc: &mut SvelteDocument, selector: &str, expected: &str) {
    let started = Instant::now();
    loop {
        doc.settle();
        doc.resolve();
        let actual = doc.query(selector).map(|node| doc.text(node));
        if actual.as_deref() == Some(expected) {
            return;
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "{selector} is {actual:?}, expected {expected:?}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// The JavaScript API, driven through the example app's buttons.
#[test]
fn the_app_drives_the_update_from_javascript() {
    let dir = scratch("js-api");
    let (private, public) = generate_keypair().unwrap();
    let server = Server::start();
    let release = dir.join("release");
    let new_build = fake_bundle(&dir.join("build"), "0.2.0");
    fs::create_dir_all(&release).unwrap();
    write_update(
        &new_build,
        &release,
        "0.2.0",
        Some("Faster"),
        &private,
        Some(&public),
    )
    .unwrap();
    server.publish(&release);

    let dist = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/updater/dist"
    ));
    let mut app = App::from_dir(dist).expect("run `pnpm run build` in the repository root first");
    // The example is built with a placeholder feed; point it at the test server.
    app.manifest.updater = Some(Config {
        url: server.feed_url(),
        pubkey: public,
    });
    let mut doc = SvelteDocument::for_app(&app).unwrap();
    doc.layout(520, 360);
    assert_eq!(doc.text(doc.query("#status").unwrap()), "idle");

    doc.click(doc.query("#check").unwrap());
    assert_eq!(
        doc.text(doc.query("#status").unwrap()),
        "checking",
        "the call does not block the UI"
    );
    wait_for_text(&mut doc, "#status", "available");
    assert_eq!(
        doc.text(doc.query("#found").unwrap()),
        "Version 0.2.0 is available. Faster"
    );

    doc.click(doc.query("#download").unwrap());
    wait_for_text(&mut doc, "#status", "ready");

    // This document does not run from an app bundle, so there is nothing to
    // replace: the promise rejects and the app shows why.
    doc.click(doc.query("#install").unwrap());
    wait_for_text(&mut doc, "#status", "error");
    assert!(
        doc.text(doc.query("#message").unwrap())
            .contains("not running from an app bundle")
    );

    // Once the server offers nothing newer, the app says so.
    server.put("/latest.json", r#"{ "version": "0.1.0", "platforms": {} }"#);
    doc.click(doc.query("#check").unwrap());
    wait_for_text(&mut doc, "#status", "up to date");
}

/// An app without `updater` in its manifest gets a clear rejection.
#[test]
fn without_updater_settings_the_api_rejects() {
    let dist = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/updater/dist"
    ));
    let mut app = App::from_dir(dist).unwrap();
    app.manifest.updater = None;
    let mut doc = SvelteDocument::for_app(&app).unwrap();
    doc.layout(520, 360);

    doc.click(doc.query("#check").unwrap());
    wait_for_text(&mut doc, "#status", "error");
    assert!(
        doc.text(doc.query("#message").unwrap())
            .contains("no `updater` in its manifest")
    );
}
