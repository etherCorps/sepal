//! The app folder, and what an app loads by URL from it.
//!
//! An app is served from `http://app.localhost/`, as a web build is from its
//! server: `index.html` is the page, and every path under that origin is a
//! file in the app folder (`dist`, or `Resources` in a bundle). The page's
//! scripts and modules, its stylesheets, fonts and images load from there,
//! and from `data:` URLs. Nothing here touches the network, and a path that
//! leaves the app folder, through `..` or a symlink, is refused.
//!
//! Loading is synchronous on purpose. The files are local and small, and it
//! means one `resolve()` after a change has everything it needs, which is what
//! makes screenshots and tests deterministic.

use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use blitz_traits::net::{Bytes, NetHandler, NetProvider, Request, Url};
use data_url::DataUrl;

/// Where the app is: the origin its page and files are served from.
pub const ORIGIN: &str = "http://app.localhost/";

/// Whether a URL is one of the app's own files.
pub fn in_app(url: &Url) -> bool {
    url.origin().ascii_serialization() == ORIGIN.trim_end_matches('/')
}

/// Why a file could not be had.
#[derive(Debug, PartialEq)]
pub enum Refused {
    /// There is no such file in the app folder: a 404.
    Missing,
    /// Anything else, with the reason.
    Other(String),
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Refused::Missing => f.write_str("not found in the app folder"),
            Refused::Other(reason) => f.write_str(reason),
        }
    }
}

/// The app folder, canonical. Every file an app loads by URL comes through here.
pub struct AppFolder {
    root: Option<PathBuf>,
    /// Files read so far, for the startup trace.
    read: AtomicUsize,
}

impl AppFolder {
    pub fn new(dir: Option<&Path>) -> Self {
        Self {
            root: dir.and_then(|dir| dir.canonicalize().ok()),
            read: AtomicUsize::new(0),
        }
    }

    /// How many files have been read from the folder.
    pub fn files_read(&self) -> usize {
        self.read.load(Ordering::Relaxed)
    }

    /// The file an `http://app.localhost/…` URL names.
    pub fn file(&self, url: &Url) -> Result<PathBuf, Refused> {
        if !in_app(url) {
            return Err(Refused::Other(format!(
                "{} is not in the app folder",
                url.origin().ascii_serialization()
            )));
        }
        let root = self
            .root
            .as_ref()
            .ok_or_else(|| Refused::Other("the app has no folder".into()))?;
        // `..` was already taken out when the URL was parsed; a `/` in the
        // path is a directory on every system. Going through a file: URL
        // does the percent-decoding.
        let base = Url::from_directory_path(root)
            .map_err(|()| Refused::Other("the app folder is not a path".into()))?;
        let path = base
            .join(url.path().trim_start_matches('/'))
            .ok()
            .and_then(|url| url.to_file_path().ok())
            .ok_or_else(|| Refused::Other("not a file path".into()))?;
        // Canonical paths on both sides, so neither `..` nor a symlink leads out.
        let path = path.canonicalize().map_err(|e| match e.kind() {
            ErrorKind::NotFound => Refused::Missing,
            _ => Refused::Other(e.to_string()),
        })?;
        if !path.starts_with(root) {
            return Err(Refused::Other("outside the app folder".into()));
        }
        // A directory is its index.html, as on a web server.
        let path = if path.is_dir() {
            path.join("index.html")
        } else {
            path
        };
        if !path.is_file() {
            return Err(Refused::Missing);
        }
        Ok(path)
    }

    /// The bytes of the file an `http://app.localhost/…` URL names.
    pub fn read(&self, url: &Url) -> Result<Vec<u8>, Refused> {
        let bytes = fs::read(self.file(url)?).map_err(|e| Refused::Other(e.to_string()))?;
        self.read.fetch_add(1, Ordering::Relaxed);
        Ok(bytes)
    }

    /// A module's source, by its URL.
    pub fn module(&self, url: &str) -> Result<String, String> {
        let url = Url::parse(url).map_err(|e| format!("{url}: {e}"))?;
        let bytes = self
            .read(&url)
            .map_err(|e| format!("cannot load {url}: {e}"))?;
        String::from_utf8(bytes).map_err(|_| format!("{url} is not UTF-8"))
    }
}

/// What Blitz loads for the page: stylesheets, fonts, images.
pub struct AppResources {
    folder: std::sync::Arc<AppFolder>,
    /// Whether each URL loaded, for the `load` and `error` events of a
    /// `<link rel=stylesheet>` the app inserts.
    outcomes: Mutex<HashMap<String, bool>>,
}

impl AppResources {
    pub fn new(app_dir: Option<&Path>) -> Self {
        Self::in_folder(std::sync::Arc::new(AppFolder::new(app_dir)))
    }

    pub fn in_folder(folder: std::sync::Arc<AppFolder>) -> Self {
        Self {
            folder,
            outcomes: Mutex::default(),
        }
    }

    pub fn folder(&self) -> &std::sync::Arc<AppFolder> {
        &self.folder
    }

    /// The URL relative paths resolve against: the app's origin, when it has a folder.
    pub fn base_url(&self) -> Option<String> {
        self.folder.root.as_ref().map(|_| ORIGIN.to_owned())
    }

    /// Whether the last request for this URL got its file. `None` if it was never asked for.
    pub fn loaded(&self, url: &str) -> Option<bool> {
        self.outcomes.lock().unwrap().get(url).copied()
    }

    fn load(&self, url: &Url) -> Result<Vec<u8>, String> {
        match url.scheme() {
            "data" => {
                let data = DataUrl::process(url.as_str()).map_err(|e| format!("{e:?}"))?;
                let (bytes, _fragment) = data.decode_to_vec().map_err(|e| format!("{e:?}"))?;
                Ok(bytes)
            }
            _ if in_app(url) => self.folder.read(url).map_err(|e| e.to_string()),
            _ => Err(
                "only data: URLs and files in the app folder load; stylesheets, scripts and fonts never come from the network"
                    .into(),
            ),
        }
    }
}

impl NetProvider for AppResources {
    fn fetch(&self, _doc_id: usize, request: Request, handler: Box<dyn NetHandler>) {
        let loaded = self.load(&request.url);
        // Not data: URLs, which can be megabytes long and always load.
        if request.url.scheme() != "data" {
            self.outcomes
                .lock()
                .unwrap()
                .insert(request.url.to_string(), loaded.is_ok());
        }
        let bytes = loaded.unwrap_or_else(|error| {
            // A data: URL can be megabytes long; the start identifies it.
            let url: String = request.url.as_str().chars().take(80).collect();
            eprintln!("[host] cannot load {url}: {error}");
            // Answer anyway, with nothing: Blitz parks later requests for the
            // same URL behind this one until it gets a reply.
            Vec::new()
        });
        handler.bytes(request.url.to_string(), Bytes::from(bytes));
    }
}
