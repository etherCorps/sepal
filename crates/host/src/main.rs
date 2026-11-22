//! `svelte-blitz-host <dist>`                         open the app in a window
//! `svelte-blitz-host --watch <dist>`                 same, and reload on every line read from stdin
//! `svelte-blitz-host --screenshot out.png <dist> [WIDTHxHEIGHT]`
//! `svelte-blitz-host --bundle <dist> [out-dir] [--sign <identity>] [--notarize <profile>]`
//!                                                    write <name>.app (default: next to <dist>, in bundle/)
//!
//! `<dist>` is the directory the Vite preset wrote (`app.js`, `app.css`,
//! `app.json`). `<app.js> [app.css]` is accepted in its place and runs without
//! a manifest. With no arguments at all, the host runs the app it is bundled
//! with (`../Resources` next to the executable, as in a macOS `.app`).

use std::path::{Path, PathBuf};
use std::{env, fs, process::ExitCode};

use svelte_blitz_host::app::{App, Manifest};
use svelte_blitz_host::bundle::bundle_macos;

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
    const USAGE: &str = "usage: svelte-blitz-host [--watch | --screenshot out.png] <dist | app.js [app.css]> [WIDTHxHEIGHT]\n       svelte-blitz-host --bundle <dist> [out-dir] [--sign <identity>] [--notarize <keychain-profile>]";

    // Launched from Finder or the Dock: no arguments, the app is inside the bundle.
    if args.is_empty()
        && let Some(resources) = bundled_resources()
    {
        return window(App::from_dir(&resources)?, None);
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
    let (app, rest) = load(args)?.ok_or(USAGE)?;

    match shot {
        Some(out) => {
            let size = rest.first().and_then(|size| parse_size(size));
            screenshot(out, &app, size.unwrap_or(app.manifest.size()))
        }
        None => {
            let args = args.to_vec();
            let reload = move || load(&args)?.map(|(app, _)| app).ok_or(USAGE.to_owned());
            window(app, watch.then(|| Box::new(reload) as _))
        }
    }
}

/// `--bundle <dist> [out-dir] [--sign <identity>] [--notarize <profile>]`.
/// `None` if `<dist>` is missing.
fn bundle(args: &[String]) -> Result<Option<()>, String> {
    let (mut positional, mut identity, mut profile) = (Vec::new(), None, None);
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--sign" => identity = Some(args.next().ok_or("--sign needs a signing identity")?),
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
    println!("{}", bundle.display());
    Ok(Some(()))
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
    resources.join("app.js").is_file().then_some(resources)
}

/// Load the app named by the leading arguments and return the rest of them.
/// `None` if there are no arguments.
fn load(args: &[String]) -> Result<Option<(App, &[String])>, String> {
    let Some((first, mut rest)) = args.split_first() else {
        return Ok(None);
    };
    let app = if Path::new(first).is_dir() {
        App::from_dir(Path::new(first))?
    } else {
        let css = match rest.split_first() {
            Some((css, after)) if parse_size(css).is_none() => {
                rest = after;
                read(css)?
            }
            _ => String::new(),
        };
        App {
            js: read(first)?,
            css,
            manifest: Manifest::default(),
            // Bare `app.js` resolves to "", which means the current directory.
            dir: Path::new(first).parent().map(|dir| {
                if dir.as_os_str().is_empty() {
                    Path::new(".")
                } else {
                    dir
                }
                .to_owned()
            }),
        }
    };
    Ok(Some((app, rest)))
}

#[cfg(feature = "window")]
fn window(app: App, reload: Option<svelte_blitz_host::window::Reload>) -> Result<(), String> {
    svelte_blitz_host::window::run(app, reload)
}

#[cfg(not(feature = "window"))]
fn window(_app: App, _reload: Option<Box<dyn Fn() -> Result<App, String>>>) -> Result<(), String> {
    Err("built without the `window` feature".into())
}

#[cfg(feature = "screenshot")]
fn screenshot(out: &str, app: &App, (width, height): (u32, u32)) -> Result<(), String> {
    use svelte_blitz_host::{SvelteDocument, screenshot::render_png};

    let mut doc = SvelteDocument::for_app(app)?;
    let png = render_png(&mut doc, width, height, 1.0);
    fs::write(out, png).map_err(|e| format!("cannot write {out}: {e}"))
}

#[cfg(not(feature = "screenshot"))]
fn screenshot(_: &str, _: &App, _: (u32, u32)) -> Result<(), String> {
    Err("built without the `screenshot` feature".into())
}
