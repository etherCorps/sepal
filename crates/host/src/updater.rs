//! Self-update for bundled apps, in the style of Tauri's updater.
//!
//! The app's manifest names a feed URL and a public key. The feed is a small
//! JSON file listing the newest version and, per platform, an archive and its
//! signature:
//!
//! ```json
//! {
//!   "version": "0.2.0",
//!   "notes": "What changed",
//!   "pub_date": "2026-11-22T12:00:00Z",
//!   "platforms": {
//!     "darwin-arm64": { "url": "Counter-0.2.0-darwin-arm64.app.tar.gz", "signature": "…" }
//!   }
//! }
//! ```
//!
//! An archive is only unpacked after its ed25519 signature has been checked
//! against the key the running app was built with. Transport security is not
//! what makes an update trusted; the signature is. HTTPS is still required, so
//! that nobody on the network can hold an app back on an old version by
//! serving a stale feed.
//!
//! `check` and `download` work on every platform. Replacing the running app
//! (`install`) is implemented for macOS `.app` bundles.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use blitz_traits::net::Url;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde_json::{Value, json};

/// The feed is a few hundred bytes. Anything much larger is not a feed.
const MAX_FEED_BYTES: u64 = 1024 * 1024;
/// Far above any real app, low enough that a wrong URL cannot fill the disk.
const MAX_ARCHIVE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// The `updater` block of the app manifest.
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// Where the feed (`latest.json`) is.
    pub url: String,
    /// The public half of the key updates are signed with, base64.
    pub pubkey: String,
}

/// A newer version found in the feed.
#[derive(Clone, Debug, PartialEq)]
pub struct Update {
    pub version: String,
    pub notes: Option<String>,
    pub date: Option<String>,
    /// Absolute URL of this platform's archive.
    pub url: String,
    /// Signature of the archive, base64.
    pub signature: String,
}

/// The name this platform goes by in a feed: `darwin-arm64`, `linux-x64-gnu`,
/// `win32-x64-msvc`. The same names the npm host package uses.
pub fn platform() -> String {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        other => other,
    };
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "x64",
        other => other,
    };
    let abi = if cfg!(target_os = "windows") {
        "-msvc"
    } else if cfg!(all(target_os = "linux", target_env = "musl")) {
        "-musl"
    } else if cfg!(target_os = "linux") {
        "-gnu"
    } else {
        ""
    };
    format!("{os}-{arch}{abi}")
}

// ---------------------------------------------------------------------------
// Keys and signatures
// ---------------------------------------------------------------------------

fn decode<const N: usize>(what: &str, text: &str) -> Result<[u8; N], String> {
    let bytes = BASE64
        .decode(text.trim())
        .map_err(|_| format!("the {what} is not valid base64"))?;
    bytes
        .try_into()
        .map_err(|_| format!("the {what} has the wrong length"))
}

/// Make a new key pair: `(private, public)`, both base64. The private key signs
/// updates and must stay secret; the public key goes into the app manifest.
pub fn generate_keypair() -> Result<(String, String), String> {
    let mut secret = [0u8; 32];
    getrandom::fill(&mut secret).map_err(|e| format!("no random numbers available: {e}"))?;
    let key = SigningKey::from_bytes(&secret);
    Ok((
        BASE64.encode(secret),
        BASE64.encode(key.verifying_key().to_bytes()),
    ))
}

/// The public key that belongs to a private key.
pub fn public_key(private_key: &str) -> Result<String, String> {
    let key = SigningKey::from_bytes(&decode("private key", private_key)?);
    Ok(BASE64.encode(key.verifying_key().to_bytes()))
}

/// Sign `data`; returns the signature as base64.
pub fn sign(private_key: &str, data: &[u8]) -> Result<String, String> {
    let key = SigningKey::from_bytes(&decode("private key", private_key)?);
    Ok(BASE64.encode(key.sign(data).to_bytes()))
}

/// Check that `signature` is `data` signed by the owner of `public_key`.
pub fn verify(public_key: &str, data: &[u8], signature: &str) -> Result<(), String> {
    let key = VerifyingKey::from_bytes(&decode("public key", public_key)?)
        .map_err(|_| "the public key is not a valid ed25519 key")?;
    let signature = Signature::from_bytes(&decode("signature", signature)?);
    key.verify_strict(data, &signature)
        .map_err(|_| "the signature does not match: the update was not signed with this app's key, or it was changed after signing".into())
}

// ---------------------------------------------------------------------------
// The feed
// ---------------------------------------------------------------------------

fn parse_version(what: &str, text: &str) -> Result<semver::Version, String> {
    let text = text.trim();
    semver::Version::parse(text.strip_prefix('v').unwrap_or(text))
        .map_err(|e| format!("{what} {text:?} is not a version: {e}"))
}

/// HTTPS only. Plain HTTP is accepted for this machine alone, which is what
/// local testing needs and nothing else can use.
fn allowed(url: &Url) -> Result<(), String> {
    let loopback = matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"));
    match url.scheme() {
        "https" => Ok(()),
        "http" if loopback => Ok(()),
        _ => Err(format!("{url} is not an https URL")),
    }
}

/// Read a feed. Returns the update it offers this platform, or `None` if the
/// app is already on that version or newer, or the feed has nothing for this
/// platform.
pub fn parse_feed(
    json: &str,
    feed_url: &str,
    current_version: &str,
    platform: &str,
) -> Result<Option<Update>, String> {
    let feed: Value =
        serde_json::from_str(json).map_err(|e| format!("the update feed is not JSON: {e}"))?;
    let version = feed["version"]
        .as_str()
        .ok_or("the update feed has no `version`")?;

    // Never move to an older or equal version, whatever the feed says.
    if parse_version("feed version", version)? <= parse_version("app version", current_version)? {
        return Ok(None);
    }
    let entry = &feed["platforms"][platform];
    if entry.is_null() {
        return Ok(None);
    }
    let (Some(url), Some(signature)) = (entry["url"].as_str(), entry["signature"].as_str()) else {
        return Err(format!(
            "the update feed's entry for {platform} needs `url` and `signature`"
        ));
    };

    // A relative URL is relative to the feed, so the files can sit side by side.
    let base = Url::parse(feed_url).map_err(|e| format!("{feed_url}: {e}"))?;
    let url = base.join(url).map_err(|e| format!("{url}: {e}"))?;
    allowed(&url)?;

    let text = |value: &Value| value.as_str().map(str::to_owned);
    Ok(Some(Update {
        version: version.to_owned(),
        notes: text(&feed["notes"]),
        date: text(&feed["pub_date"]),
        url: url.into(),
        signature: signature.to_owned(),
    }))
}

fn fetch(url: &str, limit: u64, timeout: Duration) -> Result<Vec<u8>, String> {
    allowed(&Url::parse(url).map_err(|e| format!("{url}: {e}"))?)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .build()
        .into();
    agent
        .get(url)
        .call()
        .map_err(|e| format!("cannot fetch {url}: {e}"))?
        .body_mut()
        .with_config()
        .limit(limit)
        .read_to_vec()
        .map_err(|e| format!("cannot read {url}: {e}"))
}

// ---------------------------------------------------------------------------
// The running app's side
// ---------------------------------------------------------------------------

pub struct Updater {
    config: Config,
    current_version: String,
    /// The `.app` this process runs from, if it runs from one.
    bundle: Option<PathBuf>,
    /// What the last `check` found.
    found: Mutex<Option<Update>>,
    /// A downloaded archive whose signature has been verified, with its version.
    verified: Mutex<Option<(String, Vec<u8>)>>,
}

impl Updater {
    pub fn new(config: Config, current_version: &str, bundle: Option<PathBuf>) -> Self {
        Self {
            config,
            current_version: current_version.to_owned(),
            bundle,
            found: Mutex::new(None),
            verified: Mutex::new(None),
        }
    }

    pub fn current_version(&self) -> &str {
        &self.current_version
    }

    /// Ask the feed whether there is a newer version for this platform.
    pub fn check(&self) -> Result<Option<Update>, String> {
        let feed = fetch(&self.config.url, MAX_FEED_BYTES, Duration::from_secs(30))?;
        let feed = String::from_utf8(feed).map_err(|_| "the update feed is not text")?;
        let update = parse_feed(&feed, &self.config.url, &self.current_version, &platform())?;
        *self.found.lock().unwrap() = update.clone();
        Ok(update)
    }

    /// Download the update the last `check` found and verify its signature.
    /// Nothing on disk changes.
    pub fn download(&self) -> Result<(), String> {
        let update = self
            .found
            .lock()
            .unwrap()
            .clone()
            .ok_or("there is no update to download; call check() first")?;
        if matches!(&*self.verified.lock().unwrap(), Some((version, _)) if *version == update.version)
        {
            return Ok(());
        }
        let archive = fetch(&update.url, MAX_ARCHIVE_BYTES, Duration::from_secs(30 * 60))?;
        verify(&self.config.pubkey, &archive, &update.signature)?;
        *self.verified.lock().unwrap() = Some((update.version, archive));
        Ok(())
    }

    /// Replace the running app with the downloaded update (downloading it first
    /// if that has not happened). Returns the bundle to relaunch.
    pub fn install(&self) -> Result<PathBuf, String> {
        let bundle = self.bundle.as_deref().ok_or(
            "this app is not running from an app bundle, so there is nothing to replace (updates do not install in dev)",
        )?;
        self.download()?;
        let verified = self.verified.lock().unwrap();
        let (_, archive) = verified.as_ref().ok_or("the update was not downloaded")?;
        install_archive(archive, bundle)?;
        Ok(bundle.to_owned())
    }
}

/// Unpack a verified archive and put the app it holds where `bundle` is.
///
/// The caller is responsible for the archive being trusted: this function does
/// not look at signatures.
pub fn install_archive(archive: &[u8], bundle: &Path) -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        return Err("installing updates is only implemented for macOS app bundles so far".into());
    }
    let io = |what: &str, path: &Path, e: std::io::Error| format!("{what} {}: {e}", path.display());
    let parent = bundle
        .parent()
        .ok_or("the app bundle has no parent directory")?;
    let name = bundle
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("the app bundle has no name")?;

    // Next to the app, so the final step is a rename on one volume, not a copy.
    let scratch = parent.join(format!(".{name}.update-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    let unpacked = scratch.join("new");
    fs::create_dir_all(&unpacked).map_err(|e| io("cannot create", &unpacked, e))?;

    let result = (|| {
        tar::Archive::new(GzDecoder::new(archive))
            .unpack(&unpacked)
            .map_err(|e| format!("cannot unpack the update: {e}"))?;

        // The archive holds one app. Its name inside does not matter: it takes
        // the place of the running one, wherever the user keeps it.
        let mut apps = fs::read_dir(&unpacked)
            .map_err(|e| io("cannot read", &unpacked, e))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "app") && path.is_dir());
        let (Some(new_app), None) = (apps.next(), apps.next()) else {
            return Err("the update archive does not hold exactly one .app".to_owned());
        };

        let old = scratch.join("old");
        fs::rename(bundle, &old).map_err(|e| io("cannot move aside", bundle, e))?;
        if let Err(e) = fs::rename(&new_app, bundle) {
            // Put the old app back; the user must not end up without one.
            let _ = fs::rename(&old, bundle);
            return Err(io("cannot move the new app to", bundle, e));
        }
        Ok(())
    })();

    let _ = fs::remove_dir_all(&scratch);
    result
}

/// Start the app again from its (new) bundle. The caller exits afterwards.
pub fn relaunch(bundle: &Path) -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        return Err("relaunching is only implemented for macOS app bundles so far".into());
    }
    // `-n`: a new instance, even though this one is still running for a moment.
    std::process::Command::new("open")
        .arg("-n")
        .arg(bundle)
        .status()
        .map_err(|e| format!("cannot relaunch: {e}"))?
        .success()
        .then_some(())
        .ok_or("cannot relaunch: `open` failed".into())
}

// ---------------------------------------------------------------------------
// The build side
// ---------------------------------------------------------------------------

/// Turn a built app bundle into an update: write the archive, its signature
/// and `latest.json` into `out`. Returns the path of the archive.
///
/// If `latest.json` is already there for the same version, this platform is
/// added to it, so builds on several platforms can share one feed.
///
/// `expected_public_key` is the key in the app's manifest. Signing with a key
/// that does not match it would produce updates no installed app accepts, so
/// that is an error here rather than a surprise later.
pub fn write_update(
    bundle: &Path,
    out: &Path,
    version: &str,
    notes: Option<&str>,
    private_key: &str,
    expected_public_key: Option<&str>,
) -> Result<PathBuf, String> {
    let io = |path: &Path, e: std::io::Error| format!("{}: {e}", path.display());
    parse_version("app version", version)?;
    let ours = public_key(private_key)?;
    if expected_public_key.is_some_and(|expected| expected.trim() != ours) {
        return Err(format!(
            "the signing key does not match `updater.pubkey` in the app manifest (the key's public half is {ours})"
        ));
    }

    let app_name = bundle
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("the app bundle has no name")?;
    let stem = app_name.strip_suffix(".app").unwrap_or(app_name);
    let platform = platform();
    let file_name = format!("{stem}-{version}-{platform}.app.tar.gz");

    let mut tar = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
    // Symlinks stay symlinks: a code signature covers them as they are.
    tar.follow_symlinks(false);
    tar.append_dir_all(app_name, bundle)
        .map_err(|e| io(bundle, e))?;
    let archive = tar
        .into_inner()
        .and_then(|gz| gz.finish())
        .map_err(|e| io(bundle, e))?;
    let signature = sign(private_key, &archive)?;

    fs::create_dir_all(out).map_err(|e| io(out, e))?;
    let archive_path = out.join(&file_name);
    fs::write(&archive_path, &archive).map_err(|e| io(&archive_path, e))?;
    let signature_path = out.join(format!("{file_name}.sig"));
    fs::write(&signature_path, &signature).map_err(|e| io(&signature_path, e))?;

    let feed_path = out.join("latest.json");
    let mut feed = fs::read_to_string(&feed_path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .filter(|feed| feed["version"] == version && feed["platforms"].is_object())
        .unwrap_or_else(|| json!({ "version": version, "platforms": {} }));
    if let Some(notes) = notes {
        feed["notes"] = json!(notes);
    }
    // The URL is relative: upload the three files to one place and it works.
    feed["platforms"][&platform] = json!({ "url": file_name, "signature": signature });
    let text = serde_json::to_string_pretty(&feed).map_err(|e| e.to_string())?;
    fs::write(&feed_path, text + "\n").map_err(|e| io(&feed_path, e))?;

    Ok(archive_path)
}

/// Read a whole stream; used for the private key on stdin.
pub fn read_all(mut input: impl Read) -> Result<String, String> {
    let mut text = String::new();
    input.read_to_string(&mut text).map_err(|e| e.to_string())?;
    Ok(text)
}
