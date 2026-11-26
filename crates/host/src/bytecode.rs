//! An app's modules compiled ahead of time, to QuickJS bytecode.
//!
//! `calyx --compile <dist>` (`js::compile_folder`) writes `x.js.qbc` next to
//! every module `x.js` of the app folder, and the QuickJS loader takes the
//! bytecode over the source when it fits this host. The `.js` stays: it is
//! what loads when the bytecode does not fit, and what V8 always loads.
//!
//! QuickJS does not check bytecode: a crafted file can make it run anything.
//! So bytecode is read only from the app folder, through `AppFolder`, the
//! way the modules themselves are (no `..`, no symlink out of it), never
//! from a cache or any other writable place, and only when its header says
//! this host wrote it for this module:
//!
//! ```text
//! SEPALQBC          magic
//! 1                 format of what follows
//! calyx version     u8 length, UTF-8; this host's must be the same
//! engine            u8 length, UTF-8: "QuickJS-ng 0.16.2"; the same
//! target            u8 length, UTF-8: "aarch64-apple-darwin"; the same
//! module URL        u16 length, UTF-8: what the module is loaded as
//! source length     u64: the .js it came from, checked on every launch
//! source hash       u64: FNV-1a of that .js, checked in dev and by --compile
//! payload           JS_WriteObject's output, without source text
//! ```
//!
//! The target is there because only arm64 macOS has been seen to write the
//! same bytes as the others (QuickJS writes native-endian, and nothing in a
//! module's bytecode depends on the word size), and a wrong guess would be
//! memory corruption, not an error. A bundle is compiled again by the host
//! it ships, so only an app folder copied between machines is affected.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use blitz_traits::net::Url;

use crate::net::{AppFolder, ORIGIN, Refused};

/// What a module's compiled file is called: its own name and this.
pub const SUFFIX: &str = ".qbc";
const MAGIC: &[u8; 8] = b"SEPALQBC";
const FORMAT: u8 = 1;

/// Who wrote the bytecode, and from what.
#[derive(Debug, Clone, PartialEq)]
pub struct Header {
    pub calyx: String,
    pub engine: String,
    pub target: String,
    pub url: String,
    pub source_len: u64,
    pub source_hash: u64,
}

impl Header {
    /// The header this host writes for the module at `url` with `source`.
    pub fn new(url: &str, source: &[u8]) -> Self {
        Self {
            calyx: env!("CARGO_PKG_VERSION").to_owned(),
            engine: crate::js::engine(),
            target: env!("SEPAL_TARGET").to_owned(),
            url: url.to_owned(),
            source_len: source.len() as u64,
            source_hash: hash(source),
        }
    }

    /// The header followed by `payload`, as written to a `.qbc` file.
    pub fn encode(&self, payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(payload.len() + 128);
        out.extend_from_slice(MAGIC);
        out.push(FORMAT);
        for text in [&self.calyx, &self.engine, &self.target] {
            // Versions and target names are short; a longer one is cut,
            // and then simply does not match.
            let text = &text.as_bytes()[..text.len().min(255)];
            out.push(text.len() as u8);
            out.extend_from_slice(text);
        }
        let url = &self.url.as_bytes()[..self.url.len().min(u16::MAX as usize)];
        out.extend_from_slice(&(url.len() as u16).to_le_bytes());
        out.extend_from_slice(url);
        out.extend_from_slice(&self.source_len.to_le_bytes());
        out.extend_from_slice(&self.source_hash.to_le_bytes());
        out.extend_from_slice(payload);
        out
    }

    /// Split a `.qbc` file into its header and payload.
    pub fn decode(bytes: &[u8]) -> Result<(Self, &[u8]), String> {
        let mut read = Reader(bytes);
        if read.take(MAGIC.len())? != MAGIC {
            return Err("was not written by calyx".into());
        }
        let format = read.take(1)?[0];
        if format != FORMAT {
            return Err(format!("is in format {format}; this host reads {FORMAT}"));
        }
        let header = Self {
            calyx: read.text(1)?,
            engine: read.text(1)?,
            target: read.text(1)?,
            url: read.text(2)?,
            source_len: read.u64()?,
            source_hash: read.u64()?,
        };
        Ok((header, read.0))
    }

    /// Whether this host wrote it: same calyx, engine and target.
    fn from_this_host(&self) -> bool {
        let ours = Self::new("", b"");
        self.calyx == ours.calyx && self.engine == ours.engine && self.target == ours.target
    }

    fn describe(&self) -> String {
        format!("calyx {} ({}, {})", self.calyx, self.engine, self.target)
    }
}

/// A `.qbc` header, front to back.
struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        if self.0.len() < n {
            return Err("is cut short".into());
        }
        let (head, tail) = self.0.split_at(n);
        self.0 = tail;
        Ok(head)
    }

    /// Text after a length of `width` bytes (1, or 2 little-endian).
    fn text(&mut self, width: usize) -> Result<String, String> {
        let len = match self.take(width)? {
            [len] => *len as usize,
            [low, high] => u16::from_le_bytes([*low, *high]) as usize,
            _ => unreachable!("lengths are one or two bytes"),
        };
        String::from_utf8(self.take(len)?.to_vec())
            .map_err(|_| "has a header that is not UTF-8".into())
    }

    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().expect("took 8 bytes"),
        ))
    }
}

/// FNV-1a, 64 bits: stable across Rust versions and machines, which the
/// standard library's hasher does not promise.
pub fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// The URL a file of the app folder is loaded as, from its path in the folder.
pub fn url_of(relative: &Path) -> Result<String, String> {
    let mut url = Url::parse(ORIGIN).expect("the app's origin is a URL");
    {
        let mut segments = url
            .path_segments_mut()
            .expect("the app's origin has a path");
        segments.pop_if_empty();
        for part in relative.components() {
            let part = part
                .as_os_str()
                .to_str()
                .ok_or_else(|| format!("{} is not a UTF-8 path", relative.display()))?;
            segments.push(part);
        }
    }
    Ok(url.into())
}

/// The URL of a module's compiled file.
fn compiled_url(module: &Url) -> Url {
    let mut url = module.clone();
    url.set_query(None);
    url.set_fragment(None);
    url.set_path(&format!("{}{SUFFIX}", module.path()));
    url
}

/// Why a module's bytecode is not used.
pub enum Unfit {
    /// Another host wrote it: every module of the app is in the same case.
    OtherHost(String),
    /// Something about this one file.
    File(String),
}

/// The bytecode to run for the module at `url`, if the app folder has some
/// that this host wrote for exactly that module. `None` when there is none:
/// the app was built without a calyx, or the module was left as JavaScript.
///
/// The `.js`'s size is checked against the header on every launch, a stat;
/// in `dev`, its contents too. A shipped app's `.js` and `.qbc` are written
/// by the same build.
pub fn lookup(folder: &AppFolder, url: &str, dev: bool) -> Result<Option<Vec<u8>>, Unfit> {
    let Ok(module) = Url::parse(url) else {
        return Ok(None);
    };
    let file = |reason: String| Unfit::File(format!("{}{SUFFIX} {reason}", module.path()));
    let bytes = match folder.read(&compiled_url(&module)) {
        Ok(bytes) => bytes,
        Err(Refused::Missing) => return Ok(None),
        Err(Refused::Other(reason)) => return Err(file(format!("cannot be read: {reason}"))),
    };
    let (header, payload) = Header::decode(&bytes).map_err(file)?;
    if !header.from_this_host() {
        return Err(Unfit::OtherHost(header.describe()));
    }
    if header.url != url {
        return Err(file(format!("was compiled for {}", header.url)));
    }
    let source = folder
        .file(&module)
        .map_err(|e| file(format!("has no module: {e}")))?;
    let size = std::fs::metadata(&source)
        .map(|meta| meta.len())
        .unwrap_or(u64::MAX);
    let changed = size != header.source_len
        || (dev
            && std::fs::read(&source).map(|bytes| hash(&bytes)).ok() != Some(header.source_hash));
    if changed {
        return Err(file(format!("is older than {}", module.path())));
    }
    Ok(Some(payload.to_vec()))
}

/// Whether the compiled file next to `module` (in the folder, at `url`) is
/// what `--compile` would write now. Its contents are always compared.
pub fn current(folder: &AppFolder, url: &str, source: &[u8]) -> bool {
    let Ok(module) = Url::parse(url) else {
        return false;
    };
    let Ok(bytes) = folder.read(&compiled_url(&module)) else {
        return false;
    };
    Header::decode(&bytes).is_ok_and(|(header, _)| header == Header::new(url, source))
}

/// The engine's way to ask for a module's bytecode, which reports once per
/// launch that another host's bytecode is passed over, and each file that
/// does not fit.
pub fn finder(folder: Arc<AppFolder>, dev: bool) -> crate::js::ModuleBytecode {
    let warned = Cell::new(false);
    Box::new(move |url| match lookup(&folder, url, dev) {
        Ok(found) => found,
        Err(Unfit::OtherHost(by)) => {
            if !warned.replace(true) {
                let ours = Header::new("", b"").describe();
                eprintln!(
                    "[host] the app's modules were compiled by {by}, and this is {ours}: loading their JavaScript instead. Build the app again to compile them for this host."
                );
            }
            None
        }
        Err(Unfit::File(reason)) => {
            eprintln!("[host] {reason}: loading the JavaScript instead");
            None
        }
    })
}

/// The modules (`.js`, `.mjs`) and compiled files under `root`, relative to
/// it, in a stable order.
pub fn scan(root: &Path) -> Result<(Vec<PathBuf>, Vec<PathBuf>), String> {
    let (mut modules, mut compiled) = (Vec::new(), Vec::new());
    let mut stack = vec![root.to_owned()];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
            let path = entry.path();
            let kind = entry
                .file_type()
                .map_err(|e| format!("{}: {e}", path.display()))?;
            if kind.is_dir() {
                stack.push(path);
                continue;
            }
            let relative = path
                .strip_prefix(root)
                .expect("walked from root")
                .to_owned();
            let name = relative.to_string_lossy();
            if name.ends_with(SUFFIX) {
                compiled.push(relative);
            } else if name.ends_with(".js") || name.ends_with(".mjs") {
                modules.push(relative);
            }
        }
    }
    modules.sort();
    compiled.sort();
    Ok((modules, compiled))
}

/// Write a compiled file so that a host reading the folder meanwhile (dev)
/// sees the old one or the new one, never half of one.
pub fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let partial = path.with_extension("qbc.partial");
    std::fs::write(&partial, bytes)
        .and_then(|()| std::fs::rename(&partial, path))
        .map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_header_reads_back_as_written() {
        let header = Header::new("http://app.localhost/a.js", b"export const a = 1;");
        let file = header.encode(b"payload");
        let (read, payload) = Header::decode(&file).unwrap();
        assert_eq!(read, header);
        assert_eq!(payload, b"payload");
        assert!(read.from_this_host());
    }

    #[test]
    fn a_header_cut_anywhere_is_an_error() {
        let file = Header::new("http://app.localhost/a.js", b"x").encode(b"");
        for len in 0..file.len() {
            assert!(Header::decode(&file[..len]).is_err(), "cut at {len}");
        }
        let mut other = file.clone();
        other[0] = b'X';
        assert!(
            Header::decode(&other)
                .unwrap_err()
                .contains("not written by calyx")
        );
        let mut format = file;
        format[MAGIC.len()] = 9;
        assert!(Header::decode(&format).unwrap_err().contains("format 9"));
    }

    #[test]
    fn file_urls_are_what_imports_resolve_to() {
        let base = Url::parse("http://app.localhost/_app/immutable/entry/start.js").unwrap();
        let imported = base.join("../chunks/a-B_c.js").unwrap();
        assert_eq!(
            url_of(Path::new("_app/immutable/chunks/a-B_c.js")).unwrap(),
            imported.as_str()
        );
        assert_eq!(
            url_of(Path::new("main.js")).unwrap(),
            "http://app.localhost/main.js"
        );
    }
}
