//! The desktop APIs, called from JavaScript in `examples/desktop`: fetch,
//! files, and the permission checks in front of everything. Dialogs, opening
//! links and the real clipboard are not triggered here: they would pop up on
//! whoever runs the tests.
//!
//! Build the bundle first: `pnpm run build` in the repository root.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use blitz_traits::net::Url;
use sepal::SvelteDocument;
use sepal::api::{Api, Permissions, Scope, named_dir, open_target, url_allowed};
use sepal::app::{App, Manifest};
use sepal::net::AppFolder;
use serde_json::{Value, json};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sepal-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    if cfg!(windows) {
        // With `/`, which Windows takes as well: these paths are written into
        // JavaScript strings, where a `\\` would start an escape.
        return PathBuf::from(dir.to_string_lossy().replace('\\', "/"));
    }
    // Temp directories are behind a symlink on macOS; tests compare real paths.
    dir.canonicalize().unwrap()
}

/// A file that is there, and outside anything a test allows.
const SYSTEM_FILE: &str = if cfg!(windows) {
    "C:/Windows/win.ini"
} else {
    "/etc/hosts"
};

/// The example app with the permissions a test wants.
fn app_with(permissions: Permissions) -> SvelteDocument {
    let dist = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/desktop/dist"
    ));
    let mut app = App::from_dir(dist).expect("run `pnpm run build` in the repository root first");
    app.manifest.permissions = permissions;
    let mut doc = SvelteDocument::for_app(&app).unwrap();
    doc.layout(640, 520);
    doc
}

/// Evaluate an expression in the app and wait for it. Returns the JSON of the
/// value it resolved to, or `ERROR: message` if it rejected.
fn run(doc: &mut SvelteDocument, expression: &str) -> String {
    let script = format!(
        "globalThis.__result = undefined;
         (async () => {{
             try {{ globalThis.__result = JSON.stringify(await ({expression})) ?? 'undefined'; }}
             catch (error) {{ globalThis.__result = 'ERROR: ' + error.message; }}
         }})();
         'started'"
    );
    doc.js().eval_string(&script).unwrap();
    let started = Instant::now();
    loop {
        doc.settle();
        let result = doc
            .js()
            .eval_string("String(globalThis.__result ?? '<pending>')")
            .unwrap();
        if result != "<pending>" {
            return result;
        }
        assert!(
            started.elapsed() < Duration::from_secs(15),
            "{expression} never finished"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// A small HTTP server on this machine. Returns its origin.
fn serve(other_origin: Option<String>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request_line = String::new();
            if reader.read_line(&mut request_line).is_err() {
                continue;
            }
            let mut parts = request_line.split(' ');
            let method = parts.next().unwrap_or("").to_owned();
            let path = parts.next().unwrap_or("/").to_owned();

            let (mut length, mut test_header) = (0usize, String::new());
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                    break;
                }
                let (name, value) = line.split_once(':').unwrap_or(("", ""));
                match name.to_ascii_lowercase().as_str() {
                    "content-length" => length = value.trim().parse().unwrap_or(0),
                    "x-test" => test_header = value.trim().to_owned(),
                    _ => {}
                }
            }
            let mut body = vec![0; length];
            let _ = reader.read_exact(&mut body);

            let redirect = |to: String| ("302 Found", format!("Location: {to}\r\n"), Vec::new());
            let ok = |body: Vec<u8>| ("200 OK", String::new(), body);
            let (status, extra, body) = match path.as_str() {
                "/hello" => ok(b"hello".to_vec()),
                "/json" => ok(br#"{"answer":42}"#.to_vec()),
                "/bytes" => ok(vec![0, 255, 1, 254, 128]),
                "/echo" => ok(format!(
                    "{method} [{}] {}",
                    test_header,
                    String::from_utf8_lossy(&body)
                )
                .into_bytes()),
                "/redirect" => redirect("/hello".into()),
                "/post-redirect" => (
                    "303 See Other",
                    "Location: /echo\r\n".to_owned(),
                    Vec::new(),
                ),
                "/leave" => redirect(format!("{}/hello", other_origin.as_deref().unwrap_or(""))),
                "/loop" => redirect("/loop".into()),
                "/into-app" => redirect("http://app.localhost/app.json".into()),
                _ => ("404 Not Found", String::new(), b"nothing here".to_vec()),
            };
            let head = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: text/plain\r\nX-Served: yes\r\n{extra}Connection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&body);
        }
    });
    origin
}

#[test]
fn permissions_are_read_strictly() {
    let dir = Path::new(".");
    let manifest = Manifest::parse(
        r#"{ "permissions": { "http": ["https://api.example.com"], "fs": true, "dialog": true, "open": false } }"#,
        dir,
    )
    .unwrap();
    let expected = Permissions {
        http: Scope::Some(vec!["https://api.example.com".into()]),
        fs: Scope::All,
        dialog: true,
        ..Default::default()
    };
    assert_eq!(manifest.permissions, expected);

    // No block: nothing is allowed.
    assert_eq!(
        Manifest::parse("{}", dir).unwrap().permissions,
        Permissions::default()
    );

    // A typo must not turn into a decision either way.
    for bad in [
        r#"{ "permissions": { "http": "https://api.example.com" } }"#,
        r#"{ "permissions": { "fs": ["data", 7] } }"#,
        r#"{ "permissions": { "dialog": "yes" } }"#,
        r#"{ "permissions": true }"#,
    ] {
        assert!(Manifest::parse(bad, dir).is_err(), "{bad}");
    }
}

#[test]
fn http_scopes() {
    let allowed = |patterns: &[&str], url: &str| {
        let scope = Scope::Some(patterns.iter().map(|p| p.to_string()).collect());
        url_allowed(&scope, &Url::parse(url).unwrap())
    };

    assert!(allowed(
        &["https://api.example.com"],
        "https://api.example.com/v1/users?x=1"
    ));
    assert!(allowed(
        &["https://api.example.com"],
        "https://API.example.com/"
    ));
    // Not: another scheme, port or host, including one that only starts the same.
    assert!(!allowed(
        &["https://api.example.com"],
        "http://api.example.com/"
    ));
    assert!(!allowed(
        &["https://api.example.com"],
        "https://api.example.com:8443/"
    ));
    assert!(!allowed(
        &["https://api.example.com"],
        "https://api.example.com.evil.net/"
    ));
    assert!(!allowed(
        &["https://api.example.com"],
        "https://evil.net/?https://api.example.com"
    ));
    assert!(!allowed(
        &["https://api.example.com"],
        "https://user@evil.net/"
    ));

    // A path prefix narrows it.
    assert!(allowed(
        &["https://example.com/api/"],
        "https://example.com/api/items"
    ));
    assert!(!allowed(
        &["https://example.com/api/"],
        "https://example.com/admin"
    ));

    // A wildcard covers the domain and what is under it, nothing alongside it.
    assert!(allowed(
        &["https://*.example.com"],
        "https://cdn.example.com/a.png"
    ));
    assert!(allowed(&["https://*.example.com"], "https://example.com/"));
    assert!(!allowed(
        &["https://*.example.com"],
        "https://notexample.com/"
    ));

    assert!(allowed(
        &["http://localhost:5173"],
        "http://localhost:5173/api"
    ));
    assert!(!allowed(
        &["http://localhost:5173"],
        "http://localhost:3000/api"
    ));

    let any = |url: &str| url_allowed(&Scope::All, &Url::parse(url).unwrap());
    assert!(any("https://anything.example/") && any("http://anything.example/"));
    assert!(!any("file:///etc/hosts") && !any("ftp://example.com/"));
    assert!(!url_allowed(
        &Scope::None,
        &Url::parse("https://example.com/").unwrap()
    ));
}

#[test]
fn fetch_from_javascript() {
    let elsewhere = serve(None);
    let origin = serve(Some(elsewhere.clone()));
    let mut doc = app_with(Permissions {
        http: Scope::Some(vec![origin.clone()]),
        ..Default::default()
    });

    let text = run(
        &mut doc,
        &format!(
            "fetch('{origin}/hello').then(async (r) => [r.status, r.ok, r.statusText, await r.text(), r.headers.get('X-SERVED'), r.redirected])"
        ),
    );
    assert_eq!(text, r#"[200,true,"OK","hello","yes",false]"#);

    assert_eq!(
        run(
            &mut doc,
            &format!("fetch('{origin}/json').then((r) => r.json())")
        ),
        r#"{"answer":42}"#
    );
    assert_eq!(
        run(
            &mut doc,
            &format!("fetch('{origin}/bytes').then((r) => r.bytes()).then((b) => Array.from(b))")
        ),
        "[0,255,1,254,128]"
    );
    assert_eq!(
        run(
            &mut doc,
            &format!(
                "fetch('{origin}/bytes').then((r) => r.arrayBuffer()).then((b) => b.byteLength)"
            )
        ),
        "5"
    );

    // A failed status is a response, not an exception.
    assert_eq!(
        run(
            &mut doc,
            &format!("fetch('{origin}/nope').then(async (r) => [r.status, r.ok, await r.text()])")
        ),
        r#"[404,false,"nothing here"]"#
    );

    // Method, headers and a body, as text and as bytes.
    let post = format!(
        "fetch('{origin}/echo', {{ method: 'POST', headers: {{ 'X-Test': 'abc' }}, body: 'ping' }}).then((r) => r.text())"
    );
    assert_eq!(run(&mut doc, &post), r#""POST [abc] ping""#);
    let bytes = format!(
        "fetch('{origin}/echo', {{ method: 'PUT', headers: [['x-test', 'b']], body: new Uint8Array([104, 105]) }}).then((r) => r.text())"
    );
    assert_eq!(run(&mut doc, &bytes), r#""PUT [b] hi""#);

    // Redirects are followed; a 303 after a POST becomes a GET without the body.
    assert_eq!(
        run(
            &mut doc,
            &format!(
                "fetch('{origin}/redirect').then(async (r) => [r.redirected, r.url.endsWith('/hello'), await r.text()])"
            )
        ),
        r#"[true,true,"hello"]"#
    );
    let see_other = format!(
        "fetch('{origin}/post-redirect', {{ method: 'POST', body: 'x' }}).then((r) => r.text())"
    );
    assert_eq!(run(&mut doc, &see_other), r#""GET [] ""#);
    assert!(run(&mut doc, &format!("fetch('{origin}/loop')")).contains("too many redirects"));

    // A body can be read once.
    let twice = format!(
        "fetch('{origin}/hello').then(async (r) => {{ await r.text(); return r.text(); }})"
    );
    assert_eq!(
        run(&mut doc, &twice),
        "ERROR: the response body was already read"
    );

    // Not in the list: refused, with the entry to add. So is a redirect that leaves the list.
    let refused = run(&mut doc, &format!("fetch('{elsewhere}/hello')"));
    assert_eq!(
        refused,
        format!(
            "ERROR: a request to {elsewhere} is not allowed: add it to `permissions.http` in sepal.config"
        )
    );
    assert!(run(&mut doc, &format!("fetch('{origin}/leave')")).contains("is not allowed"));
    // `app.localhost` is this machine: no server may send the app there.
    assert!(run(&mut doc, &format!("fetch('{origin}/into-app')")).contains("the app's own origin"),);

    // The app's own files, by path, with no permission for them.
    assert_eq!(
        run(
            &mut doc,
            "fetch('/app.json').then(async (r) => [r.status, r.headers.get('content-type'), (await r.json()).name])"
        ),
        r#"[200,"application/json","Desktop APIs"]"#
    );
    assert_eq!(
        run(
            &mut doc,
            "fetch('/no-such-file.json').then((r) => [r.status, r.ok])"
        ),
        "[404,false]"
    );
    assert!(run(&mut doc, "fetch('file:///etc/hosts')").contains("is not allowed"));
    // As in a browser, that is a path relative to the page: not there.
    assert_eq!(
        run(&mut doc, "fetch('not a url').then((r) => r.status)"),
        "404"
    );
    assert!(run(&mut doc, "fetch('http://[')").starts_with("ERROR:"));

    // Without any http permission nothing goes out at all.
    let mut closed = app_with(Permissions::default());
    assert!(run(&mut closed, &format!("fetch('{origin}/hello')")).contains("permissions.http"));
}

/// The app's own origin is its folder, served as a static web server would:
/// no permission needed, read-only, and a 404 for what is not there or lies
/// outside it, whichever way the path tries to get there.
#[test]
fn same_origin_requests_are_answered_from_the_app_folder() {
    let dir = scratch("same-origin");
    let app_dir = dir.join("app");
    fs::create_dir_all(app_dir.join("_app")).unwrap();
    fs::write(app_dir.join("_app/version.json"), r#"{"version":"1"}"#).unwrap();
    fs::write(app_dir.join("index.html"), "<p>the page</p>").unwrap();
    fs::write(dir.join("secret.txt"), "secret").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(dir.join("secret.txt"), app_dir.join("link.txt")).unwrap();

    let folder = Arc::new(AppFolder::new(Some(&app_dir)));
    let api = Arc::new(Api::new(Permissions::default(), None).with_folder(folder));
    let call = |name: &str, args: Value| -> Value {
        let work = api.call(name, &args.to_string()).unwrap();
        serde_json::from_str(&work().unwrap()).unwrap()
    };
    let fetch = |url: &str, method: &str, base: Option<&str>| {
        call(
            "http.fetch",
            json!({ "url": url, "method": method, "base": base }),
        )
    };
    let body = |head: &Value| call("http.body", json!({ "id": head["id"] }));

    let head = fetch("/_app/version.json", "GET", None);
    assert_eq!(head["status"], 200);
    assert_eq!(head["url"], "http://app.localhost/_app/version.json");
    assert_eq!(body(&head), r#"{"version":"1"}"#);
    // The same file by its full URL, and relative to the page the app is on.
    assert_eq!(
        fetch("http://app.localhost/_app/version.json", "GET", None)["status"],
        200
    );
    assert_eq!(
        fetch(
            "version.json",
            "GET",
            Some("http://app.localhost/_app/page")
        )["status"],
        200
    );
    // A directory is its index.html.
    assert_eq!(body(&fetch("/", "GET", None)), "<p>the page</p>");
    // HEAD has the headers and no body.
    let head = fetch("/_app/version.json", "HEAD", None);
    assert_eq!(
        (head["status"].clone(), body(&head)),
        (json!(200), json!(""))
    );

    assert_eq!(fetch("/missing.json", "GET", None)["status"], 404);
    // Out of the folder: `..` stops at the origin's root, and a symlink that
    // leads out is as good as missing.
    for path in [
        "/../secret.txt",
        "/_app/../../secret.txt",
        "/%2e%2e/secret.txt",
        "/link.txt",
    ] {
        assert_eq!(fetch(path, "GET", None)["status"], 404, "{path}");
    }
    // Read-only.
    let post = fetch("/_app/version.json", "POST", None);
    assert_eq!(post["status"], 405);
    assert_eq!(post["headers"], json!([["allow", "GET, HEAD"]]));

    // Another origin still needs `permissions.http`.
    let refused = api
        .call(
            "http.fetch",
            &json!({ "url": "https://example.com/" }).to_string(),
        )
        .unwrap()()
    .unwrap_err();
    assert!(refused.contains("`permissions.http`"), "{refused}");
}

#[test]
fn files_from_javascript() {
    let allowed = scratch("fs-allowed");
    let outside = scratch("fs-outside");
    fs::write(outside.join("secret.txt"), "secret").unwrap();
    let mut doc = app_with(Permissions {
        fs: Scope::Some(vec![allowed.to_string_lossy().into_owned()]),
        ..Default::default()
    });
    let path = |name: &str| format!("'{}/{name}'", allowed.display());

    // Text, with parent directories created on the way.
    assert_eq!(
        run(
            &mut doc,
            &format!("__api.fs.writeTextFile({}, 'héllo ✓')", path("notes/a.txt"))
        ),
        "null"
    );
    assert_eq!(
        fs::read_to_string(allowed.join("notes/a.txt")).unwrap(),
        "héllo ✓"
    );
    assert_eq!(
        run(
            &mut doc,
            &format!("__api.fs.readTextFile({})", path("notes/a.txt"))
        ),
        r#""héllo ✓""#
    );

    // Bytes, every value, through base64 both ways.
    let all = format!(
        "__api.fs.writeFile({}, Uint8Array.from({{ length: 256 }}, (_, i) => i))",
        path("all.bin")
    );
    assert_eq!(run(&mut doc, &all), "null");
    assert_eq!(
        fs::read(allowed.join("all.bin")).unwrap(),
        (0..=255).collect::<Vec<u8>>()
    );
    let sum = format!(
        "__api.fs.readFile({}).then((b) => [b.length, b[0], b[255], b.reduce((a, x) => a + x, 0)])",
        path("all.bin")
    );
    assert_eq!(run(&mut doc, &sum), "[256,0,255,32640]");
    for length in [0, 1, 2, 3, 4] {
        let round = format!(
            "__api.fs.writeFile({0}, new Uint8Array({length}).fill(7)).then(() => __api.fs.readFile({0})).then((b) => b.length)",
            path("pad.bin")
        );
        assert_eq!(run(&mut doc, &round), length.to_string());
    }

    assert_eq!(
        run(&mut doc, &format!("__api.fs.exists({})", path("all.bin"))),
        "true"
    );
    assert_eq!(
        run(&mut doc, &format!("__api.fs.exists({})", path("nope"))),
        "false"
    );
    assert_eq!(
        run(&mut doc, &format!("__api.fs.mkdir({})", path("deep/er"))),
        "null"
    );
    assert_eq!(
        run(
            &mut doc,
            &format!("__api.fs.readDir('{}')", allowed.display())
        ),
        r#"[{"isDirectory":false,"name":"all.bin"},{"isDirectory":true,"name":"deep"},{"isDirectory":true,"name":"notes"},{"isDirectory":false,"name":"pad.bin"}]"#
    );

    // A directory with something in it is only removed when asked to.
    assert!(run(&mut doc, &format!("__api.fs.remove({})", path("notes"))).starts_with("ERROR:"));
    assert_eq!(
        run(
            &mut doc,
            &format!("__api.fs.remove({}, {{ recursive: true }})", path("notes"))
        ),
        "null"
    );
    assert_eq!(
        run(&mut doc, &format!("__api.fs.remove({})", path("all.bin"))),
        "null"
    );
    assert!(!allowed.join("notes").exists() && !allowed.join("all.bin").exists());
    assert!(
        run(
            &mut doc,
            &format!("__api.fs.readTextFile({})", path("missing.txt"))
        )
        .starts_with("ERROR:")
    );

    // Outside the allowed directory: by absolute path, by `..`, and through a symlink.
    let secret = format!("'{}/secret.txt'", outside.display());
    let refused = run(&mut doc, &format!("__api.fs.readTextFile({secret})"));
    assert!(
        refused.contains("is not allowed: add it to `permissions.fs`"),
        "{refused}"
    );
    let dotted = format!(
        "'{}/../{}/secret.txt'",
        allowed.display(),
        outside.file_name().unwrap().to_str().unwrap()
    );
    assert!(run(&mut doc, &format!("__api.fs.readTextFile({dotted})")).contains("is not allowed"));
    assert!(
        run(
            &mut doc,
            &format!(
                "__api.fs.writeTextFile('{}/new.txt', 'x')",
                outside.display()
            )
        )
        .contains("is not allowed")
    );
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&outside, allowed.join("way-out")).unwrap();
        assert!(
            run(
                &mut doc,
                &format!("__api.fs.readTextFile({})", path("way-out/secret.txt"))
            )
            .contains("is not allowed")
        );
        assert!(
            run(
                &mut doc,
                &format!("__api.fs.writeTextFile({}, 'x')", path("way-out/new.txt"))
            )
            .contains("is not allowed")
        );
    }
    assert_eq!(
        fs::read_to_string(outside.join("secret.txt")).unwrap(),
        "secret"
    );
    assert!(!outside.join("new.txt").exists());

    // Relative paths need a named directory, and cannot climb out of it.
    assert!(run(&mut doc, "__api.fs.readTextFile('notes.txt')").contains("is relative"));
    assert!(
        run(&mut doc, "__api.fs.readTextFile('../x', { dir: 'data' })")
            .contains("must be a path inside")
    );
    assert!(
        run(&mut doc, "__api.fs.readTextFile('x', { dir: 'nowhere' })")
            .contains("is not a directory name")
    );

    // The data directory is named after the app, and asking where it is needs no permission.
    let data = run(&mut doc, "__api.fs.dir('data')");
    assert!(data.contains("com.example.desktop-apis"), "{data}");
    // ...but using it does, and this app was only given the scratch directory.
    assert!(
        run(
            &mut doc,
            "__api.fs.exists('settings.json', { dir: 'data' })"
        )
        .contains("is not allowed")
    );
}

#[test]
fn named_directories() {
    let data = named_dir("data", Some("com.example.app")).unwrap();
    assert!(data.is_absolute() && data.ends_with("com.example.app"));
    assert!(named_dir("data", None).is_err(), "no id, no data directory");
    for name in ["home", "documents", "downloads", "desktop"] {
        assert!(named_dir(name, None).unwrap().is_absolute(), "{name}");
    }
    assert!(named_dir("../etc", Some("x")).is_err());
}

#[test]
fn what_may_be_opened() {
    let allowed = scratch("open-allowed");
    fs::write(allowed.join("report.pdf"), "").unwrap();
    let api = |open: bool| {
        let permissions = Permissions {
            open,
            fs: Scope::Some(vec![allowed.to_string_lossy().into_owned()]),
            ..Default::default()
        };
        Api::new(permissions, None)
    };
    let open = api(true);

    assert_eq!(
        open_target(&open, "https://svelte.dev/docs").unwrap(),
        "https://svelte.dev/docs"
    );
    assert_eq!(
        open_target(&open, "mailto:someone@example.com").unwrap(),
        "mailto:someone@example.com"
    );
    let file = allowed.join("report.pdf");
    assert_eq!(
        open_target(&open, file.to_str().unwrap()).unwrap(),
        file.to_str().unwrap()
    );

    // Other schemes are how one app makes another do things: refused.
    for target in [
        "file:///etc/hosts",
        "javascript:alert(1)",
        "x-apple.systempreferences:",
        "tel:123",
        "ssh://host",
    ] {
        let error = open_target(&open, target).err().unwrap();
        assert!(error.contains("cannot be opened"), "{target}: {error}");
    }
    // A file outside what the app may read, and something that is neither.
    assert!(
        open_target(&open, SYSTEM_FILE)
            .err()
            .unwrap()
            .contains("permissions.fs")
    );
    assert!(open_target(&open, "report.pdf").is_err());

    assert!(
        open_target(&api(false), "https://svelte.dev")
            .err()
            .unwrap()
            .contains("permissions.open")
    );
}

#[test]
fn everything_is_refused_without_permission() {
    let mut doc = app_with(Permissions::default());
    let read = format!("__api.fs.readTextFile('{SYSTEM_FILE}')");
    for (call, permission) in [
        ("__api.dialog.open()", "dialog"),
        ("__api.dialog.save()", "dialog"),
        ("__api.dialog.confirm('?')", "dialog"),
        ("__api.dialog.message('!')", "dialog"),
        ("__api.clipboard.readText()", "clipboard"),
        ("__api.clipboard.writeText('x')", "clipboard"),
        ("__api.shell.open('https://svelte.dev')", "open"),
        ("__api.window.minimize()", "window"),
        ("__api.window.close()", "window"),
        (read.as_str(), "fs"),
        ("fetch('https://example.com')", "http"),
    ] {
        let result = run(&mut doc, call);
        assert!(
            result.starts_with("ERROR:") && result.contains(&format!("`permissions.{permission}`")),
            "{call}: {result}"
        );
    }
    // Except the title, which is the app's own, as a page's is on the web.
    let result = run(&mut doc, "__api.window.setTitle('x')");
    assert!(!result.starts_with("ERROR:"), "setTitle: {result}");
}

/// With permission, the window and clipboard calls reach the shell. There is
/// no window in a test, so the shell does nothing; what is checked is that the
/// calls are wired through and answer.
#[test]
fn window_and_clipboard_calls_reach_the_shell() {
    let mut doc = app_with(Permissions {
        window: true,
        clipboard: true,
        ..Default::default()
    });
    for call in [
        "setTitle('Hello')",
        "minimize()",
        "maximize()",
        "unmaximize()",
        "setDecorations(false)",
        "startDragging()",
        "close()",
    ] {
        assert_eq!(
            run(&mut doc, &format!("__api.window.{call}")),
            "null",
            "{call}"
        );
    }
    assert_eq!(run(&mut doc, "__api.window.isMaximized()"), "false");
    // No window, so no clipboard: an error the app can show, not a crash.
    assert!(run(&mut doc, "__api.clipboard.readText()").contains("clipboard is not available"));
}

/// A slow request must not hold up the app: the UI thread keeps turning.
#[test]
fn a_request_does_not_block_the_ui() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
    let release = Arc::new(std::sync::Barrier::new(2));
    let gate = release.clone();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        // Take the request in first: Windows resets a connection that is
        // closed with unread data on it, and the client then sees an error.
        let _ = stream.read(&mut [0; 4096]);
        gate.wait();
        let _ = stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nslow");
    });

    let mut doc = app_with(Permissions {
        http: Scope::All,
        ..Default::default()
    });
    doc.js()
        .eval_string(&format!("globalThis.__slow = 'waiting'; fetch('{origin}/').then((r) => r.text()).then((t) => (globalThis.__slow = t)); ''"))
        .unwrap();

    // The server has not answered. Turns still run, and the app still reacts.
    for _ in 0..3 {
        doc.settle();
    }
    assert_eq!(
        doc.js().eval_string("globalThis.__slow").unwrap(),
        "waiting"
    );
    doc.click(doc.query("#output").unwrap());

    release.wait();
    let started = Instant::now();
    while doc.js().eval_string("globalThis.__slow").unwrap() != "slow" {
        assert!(started.elapsed() < Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(5));
        doc.settle();
    }
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

/// A file read at module top level, while the app starts and there is no
/// window yet, is answered without waiting for one; a window call made in
/// the same breath waits for the window. Headless, "the window exists"
/// is the first poll with a task context, as in `timers_wake_the_event_loop`.
#[test]
fn calls_that_need_no_window_start_before_it_exists() {
    use blitz_dom::Document;
    use std::task::{Context, Waker};

    let data = scratch("early-calls-data");
    fs::write(data.join("early.txt"), "read early").unwrap();
    let app_dir = scratch("early-calls");
    copy_dir(
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/desktop/dist"
        )),
        &app_dir,
    );
    // After the app's own module, which sets `__api`: modules run in order.
    let file = data.join("early.txt");
    let page = fs::read_to_string(app_dir.join("index.html")).unwrap().replace(
        "</head>",
        &format!(
            "<script type=\"module\">
                globalThis.early = {{ fs: 'pending', window: 'pending' }};
                __api.fs.readTextFile({file:?}).then((text) => (early.fs = text), (error) => (early.fs = 'ERROR: ' + error.message));
                __api.window.setTitle('early').then(() => (early.window = 'done'), (error) => (early.window = 'ERROR: ' + error.message));
            </script>
            </head>"
        ),
    );
    fs::write(app_dir.join("index.html"), page).unwrap();
    let mut app = App::from_dir(&app_dir).unwrap();
    app.manifest.permissions = Permissions {
        fs: Scope::Some(vec![data.to_string_lossy().into_owned()]),
        ..Default::default()
    };
    let mut doc = SvelteDocument::for_window(&app).unwrap();
    let early = |doc: &SvelteDocument| -> Value {
        serde_json::from_str(&doc.js().eval_string("JSON.stringify(early)").unwrap()).unwrap()
    };

    // Turns, but no poll: the window does not exist.
    let started = Instant::now();
    while early(&doc)["fs"] == "pending" {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the file was not read before the window existed"
        );
        std::thread::sleep(Duration::from_millis(5));
        doc.settle();
    }
    assert_eq!(early(&doc)["fs"], "read early");
    assert_eq!(early(&doc)["window"], "pending", "the window call waits");

    // The window polls the document for the first time: the held call starts.
    doc.poll(Some(Context::from_waker(Waker::noop())));
    let started = Instant::now();
    while early(&doc)["window"] == "pending" {
        assert!(started.elapsed() < Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(5));
        doc.settle();
    }
    assert_eq!(early(&doc)["window"], "done");
}
