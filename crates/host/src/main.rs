//! `calyx <dist>`                         open the app in a window
//! `calyx --watch <dist>`                 same, and reload on every line read from stdin
//! `calyx --screenshot out.png <dist> [WIDTHxHEIGHT]`
//! `calyx --bundle <dist> [out-dir] [--sign <identity>] [--notarize <profile>]
//!        [--update-key <file>] [--update-notes <text>]`
//!                                        write <name>.app (default: next to <dist>, in bundle/),
//!                                        and with a key also the signed update for it
//! `calyx --updater-keygen <file>`        make an update signing key; prints the public half
//! `calyx --version`                      the version, the same as the npm package it ships in
//!
//! `<dist>` is the app folder the Vite preset wrote: `index.html`, the
//! modules and stylesheets it loads, other files, and `app.json`. With no
//! arguments at all, the host runs the app it is bundled with
//! (`../Resources` next to the executable, as in a macOS `.app`).

use std::path::{Path, PathBuf};
use std::{env, fs, process::ExitCode};

use sepal::app::App;
use sepal::bundle::bundle_macos;
use sepal::updater;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn read(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))
}

fn parse_size(size: &str) -> Option<(u32, u32)> {
    let (width, height) = size.split_once('x')?;
    Some((width.parse().ok()?, height.parse().ok()?))
}

fn run(args: &[String]) -> Result<(), String> {
    const USAGE: &str = "usage: calyx [--watch | --screenshot out.png] <dist> [WIDTHxHEIGHT]\n       calyx --bundle <dist> [out-dir] [--sign <identity>] [--notarize <keychain-profile>] [--update-key <file>] [--update-notes <text>]\n       calyx --updater-keygen <private-key-file>\n       calyx --version";

    // Launched from Finder or the Dock: no arguments, the app is inside the bundle.
    if args.is_empty()
        && let Some(resources) = bundled_resources()
    {
        let mut app = App::from_dir(&resources)?;
        // Resources -> Contents -> Name.app
        app.bundle = resources
            .parent()
            .and_then(Path::parent)
            .map(Path::to_owned);
        return window(app, None);
    }

    if let [flag] = args
        && flag == "--version"
    {
        // scripts/check-versions.js keeps this equal to the npm packages' version.
        println!("calyx {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if let [flag, rest @ ..] = args
        && flag == "--updater-keygen"
    {
        return keygen(rest.first().ok_or(USAGE)?);
    }

    if let [flag, rest @ ..] = args
        && flag == "--bundle"
    {
        return bundle(rest).and_then(|done| done.ok_or(USAGE.to_owned()));
    }

    let (shot, watch, args) = match args {
        [flag, out, rest @ ..] if flag == "--screenshot" => (Some(out), false, rest),
        [flag, rest @ ..] if flag == "--watch" => (None, true, rest),
        _ => (None, false, args),
    };
    let (mut app, rest) = load(args)?.ok_or(USAGE)?;
    app.dev = watch;

    match shot {
        Some(out) => {
            let size = rest.first().and_then(|size| parse_size(size));
            screenshot(out, &app, size.unwrap_or(app.manifest.size()))
        }
        None => {
            let args = args.to_vec();
            let reload = move || {
                let (mut app, _) = load(&args)?.ok_or(USAGE.to_owned())?;
                app.dev = true;
                Ok(app)
            };
            window(app, watch.then(|| Box::new(reload) as _))
        }
    }
}

/// `--bundle <dist> [out-dir] [--sign <identity>] [--notarize <profile>]`.
/// `None` if `<dist>` is missing.
fn bundle(args: &[String]) -> Result<Option<()>, String> {
    let (mut positional, mut identity, mut profile) = (Vec::new(), None, None);
    let (mut update_key, mut update_notes) = (None, None);
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--sign" => identity = Some(args.next().ok_or("--sign needs a signing identity")?),
            "--update-key" => {
                update_key = Some(args.next().ok_or("--update-key needs a private key file")?)
            }
            "--update-notes" => {
                update_notes = Some(args.next().ok_or("--update-notes needs the text")?)
            }
            "--notarize" => {
                profile = Some(args.next().ok_or("--notarize needs a keychain profile")?)
            }
            _ => positional.push(arg),
        }
    }
    let Some(dist) = positional.first().map(Path::new) else {
        return Ok(None);
    };
    if profile.is_some() && identity.is_none() {
        return Err(
            "--notarize needs --sign: Apple only notarises Developer ID signed apps".into(),
        );
    }
    let out = match positional.get(1) {
        Some(out) => PathBuf::from(out),
        None => dist.parent().unwrap_or(Path::new("")).join("bundle"),
    };

    let host = env::current_exe().map_err(|e| format!("cannot find the host binary: {e}"))?;
    let bundle = bundle_macos(dist, &out, &host)?;
    sign(&bundle, identity.map(String::as_str))?;
    if let Some(profile) = profile {
        notarize(&bundle, profile)?;
    }

    // After signing and notarising: the update has to carry the finished app.
    // The key comes from a file, or from the environment for CI secrets.
    let private_key = match update_key {
        Some(file) => Some(read(file)?),
        None => env::var("SEPAL_UPDATER_KEY").ok(),
    };
    if let Some(private_key) = private_key {
        let manifest = App::from_dir(dist)?.manifest;
        let version = manifest.version.as_deref().unwrap_or("0.1.0");
        let pubkey = manifest
            .updater
            .as_ref()
            .map(|config| config.pubkey.as_str());
        let archive = updater::write_update(
            &bundle,
            &out,
            version,
            update_notes.map(String::as_str),
            &private_key,
            pubkey,
        )?;
        // stderr: stdout is the bundle path and nothing else.
        eprintln!("update: {} (+ .sig, latest.json)", archive.display());
    }
    println!("{}", bundle.display());
    Ok(Some(()))
}

/// `--updater-keygen <file>`: write a new private key to `file` and print the
/// public key that goes into the app manifest.
fn keygen(file: &str) -> Result<(), String> {
    use std::io::Write;

    let (private_key, public_key) = updater::generate_keypair()?;
    // Never overwrite: replacing the key would cut every installed app off from updates.
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut out = options
        .open(file)
        .map_err(|e| format!("cannot create {file}: {e}"))?;
    writeln!(out, "{private_key}").map_err(|e| format!("cannot write {file}: {e}"))?;

    eprintln!(
        "Private key written to {file}. Keep it secret and backed up: without it you cannot ship updates.\nPublic key, for `updater.pubkey` in the app manifest:"
    );
    println!("{public_key}");
    Ok(())
}

/// Run a tool and fail with its name if it does. Its output goes to stderr,
/// so stdout stays the bundle path.
#[cfg(target_os = "macos")]
fn tool(command: &mut std::process::Command) -> Result<(), String> {
    let name = command.get_program().to_string_lossy().into_owned();
    let status = command
        .stdout(std::process::Stdio::from(std::io::stderr()))
        .status()
        .map_err(|e| format!("cannot run {name}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{name} failed ({status})"))
    }
}

/// Sign the bundle as a whole. The copied binary only carries the linker's
/// signature, which covers the executable but not the bundle around it, so
/// `codesign --verify` rejects the app until this has run.
///
/// Without an identity the signature is ad-hoc: enough for the Mac that built
/// it. With a "Developer ID Application" identity the app can be notarised
/// and then opens on other Macs; that needs the hardened runtime and a
/// timestamp, which is what the extra flags are.
#[cfg(target_os = "macos")]
fn sign(bundle: &Path, identity: Option<&str>) -> Result<(), String> {
    let mut codesign = std::process::Command::new("codesign");
    codesign.arg("--force");
    if identity.is_some() {
        codesign.args(["--options", "runtime", "--timestamp"]);
    }
    tool(
        codesign
            .args(["--sign", identity.unwrap_or("-")])
            .arg(bundle),
    )
}

/// Send the signed bundle to Apple, wait for the verdict, and staple the
/// ticket to it so it opens offline. `profile` is a keychain profile made with
/// `xcrun notarytool store-credentials`.
#[cfg(target_os = "macos")]
fn notarize(bundle: &Path, profile: &str) -> Result<(), String> {
    use std::process::Command;

    // notarytool takes an archive, not a directory.
    let zip = bundle.with_extension("zip");
    tool(
        Command::new("ditto")
            .args(["-c", "-k", "--keepParent"])
            .args([bundle, &zip]),
    )?;
    let submitted = tool(
        Command::new("xcrun")
            .args(["notarytool", "submit"])
            .arg(&zip)
            .args(["--keychain-profile", profile, "--wait"]),
    );
    let _ = fs::remove_file(&zip);
    submitted?;
    tool(
        Command::new("xcrun")
            .args(["stapler", "staple"])
            .arg(bundle),
    )
}

#[cfg(not(target_os = "macos"))]
fn sign(_bundle: &Path, identity: Option<&str>) -> Result<(), String> {
    match identity {
        // Nothing to do: an unsigned bundle is all another platform can produce.
        None => Ok(()),
        Some(_) => Err("--sign needs macOS (codesign)".into()),
    }
}

#[cfg(not(target_os = "macos"))]
fn notarize(_bundle: &Path, _profile: &str) -> Result<(), String> {
    Err("--notarize needs macOS (notarytool)".into())
}

/// `Contents/Resources` of the bundle this executable lives in, if it is in one.
fn bundled_resources() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let resources = exe.parent()?.parent()?.join("Resources");
    // ponytail: app.js is a bundle from before index.html; it goes when every build writes index.html.
    ["index.html", "app.js"]
        .iter()
        .any(|page| resources.join(page).is_file())
        .then_some(resources)
}

/// Load the app folder named by the first argument and return the rest of
/// them. `None` if there are no arguments.
fn load(args: &[String]) -> Result<Option<(App, &[String])>, String> {
    let Some((first, rest)) = args.split_first() else {
        return Ok(None);
    };
    if !Path::new(first).is_dir() {
        return Err(format!(
            "{first} is not an app folder: give the directory the build wrote, with index.html in it"
        ));
    }
    Ok(Some((App::from_dir(Path::new(first))?, rest)))
}

#[cfg(feature = "window")]
fn window(app: App, reload: Option<sepal::window::Reload>) -> Result<(), String> {
    sepal::window::run(app, reload)
}

#[cfg(not(feature = "window"))]
fn window(_app: App, _reload: Option<Box<dyn Fn() -> Result<App, String>>>) -> Result<(), String> {
    Err("built without the `window` feature".into())
}

#[cfg(feature = "screenshot")]
fn screenshot(out: &str, app: &App, (width, height): (u32, u32)) -> Result<(), String> {
    use sepal::{SvelteDocument, screenshot::render_png};

    let mut doc = SvelteDocument::for_app(app)?;
    let png = render_png(&mut doc, width, height, 1.0);
    fs::write(out, png).map_err(|e| format!("cannot write {out}: {e}"))
}

#[cfg(not(feature = "screenshot"))]
fn screenshot(_: &str, _: &App, _: (u32, u32)) -> Result<(), String> {
    Err("built without the `screenshot` feature".into())
}
