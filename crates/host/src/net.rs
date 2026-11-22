//! Loads what an app refers to by URL: `<img src>`, CSS `url()`, `@font-face`.
//!
//! Two sources only: `data:` URLs, and files inside the app's own directory.
//! There is no network access, and a path that leaves the app directory is
//! refused.
//!
//! Loading is synchronous on purpose. The files are local and small, and it
//! means one `resolve()` after a change has everything it needs, which is what
//! makes screenshots and tests deterministic.

use std::fs;
use std::path::{Path, PathBuf};

use blitz_traits::net::{Bytes, NetHandler, NetProvider, Request, Url};
use data_url::DataUrl;

pub struct AppResources {
    /// The app directory, canonical. `None` for an app that was not loaded from disk.
    root: Option<PathBuf>,
}

impl AppResources {
    pub fn new(app_dir: Option<&Path>) -> Self {
        Self {
            root: app_dir.and_then(|dir| dir.canonicalize().ok()),
        }
    }

    /// The URL relative paths resolve against: the app directory, as `file:///…/`.
    pub fn base_url(&self) -> Option<String> {
        let root = self.root.as_ref()?;
        Url::from_directory_path(root).ok().map(String::from)
    }

    fn load(&self, url: &Url) -> Result<Vec<u8>, String> {
        match url.scheme() {
            "data" => {
                let data = DataUrl::process(url.as_str()).map_err(|e| format!("{e:?}"))?;
                let (bytes, _fragment) = data.decode_to_vec().map_err(|e| format!("{e:?}"))?;
                Ok(bytes)
            }
            "file" => {
                let root = self.root.as_ref().ok_or("the app has no directory")?;
                let base = Url::from_directory_path(root).map_err(|()| "not a file path")?;
                // Where in the app directory the URL points, worked out on the
                // URLs: file paths differ too much between systems to compare.
                //
                // `/logo.svg` is the app's logo.svg, as it would be on a web
                // server, not a file at the top of the disk. Bundlers write
                // asset addresses that way. On Windows such a URL keeps the
                // drive of the page it was on (`/D:/logo.svg`).
                let inside = url.path().strip_prefix(base.path()).unwrap_or_else(|| {
                    let rest = url.path().trim_start_matches('/');
                    match rest.as_bytes() {
                        [drive, b':', b'/', ..] if cfg!(windows) && drive.is_ascii_alphabetic() => {
                            &rest[3..]
                        }
                        _ => rest,
                    }
                });
                let path = base
                    .join(inside)
                    .ok()
                    .and_then(|url| url.to_file_path().ok())
                    .ok_or("not a file path")?;
                // Canonical paths on both sides, so neither `..` nor a symlink leads out.
                let path = path.canonicalize().map_err(|e| e.to_string())?;
                if !path.starts_with(root) {
                    return Err("outside the app directory".into());
                }
                fs::read(&path).map_err(|e| e.to_string())
            }
            scheme => Err(format!(
                "{scheme}: URLs are not supported, only data: and files in the app directory"
            )),
        }
    }
}

impl NetProvider for AppResources {
    fn fetch(&self, _doc_id: usize, request: Request, handler: Box<dyn NetHandler>) {
        let bytes = self.load(&request.url).unwrap_or_else(|error| {
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
