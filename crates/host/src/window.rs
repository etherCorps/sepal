//! Open a native window for a [`SvelteDocument`].
//!
//! The event loop comes from `blitz-shell` and runs with `ControlFlow::Wait`:
//! the process sleeps in the OS until there is input, a timer fires, or a
//! redraw was requested. Nothing is painted unless something changed.
//!
//! In dev (`--watch`) the host also reads lines from stdin, one per rebuild,
//! and swaps in a freshly loaded document without closing the window.

use std::io::BufRead;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Instant;

use anyrender_vello_hybrid::VelloHybridWindowRenderer;
use blitz_dom::DocumentConfig;
use blitz_shell::{BlitzApplication, BlitzShellProxy, WindowConfig, create_default_event_loop};
use winit::application::ApplicationHandler;
use winit::application::macos::ApplicationHandlerExtMacOS;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::icon::RgbaIcon;
use winit::window::{WindowAttributes, WindowId};

use crate::SvelteDocument;
use crate::app::App;

/// Loads the app again from wherever it came from. Present in dev only.
pub type Reload = Box<dyn Fn() -> Result<App, String>>;

enum Dev {
    Reload,
    Quit,
}

/// Run an app until its window is closed. With `reload`, also follow stdin:
/// every line reloads the app, end of input quits.
pub fn run(app: App, reload: Option<Reload>) -> Result<(), String> {
    let App { js, css, manifest } = app;
    // Before the event loop exists: winit builds the menu bar from this name.
    if let Some(name) = &manifest.name {
        set_app_name(name);
    }
    let event_loop = create_default_event_loop();

    let (dev_sender, dev_events) = mpsc::channel();
    if reload.is_some() {
        let wake = event_loop.create_proxy();
        thread::Builder::new()
            .name("svelte-blitz-dev".into())
            .spawn(move || {
                // Blocks in read(), so this costs nothing between rebuilds.
                for line in std::io::stdin().lock().lines() {
                    if line.is_err() || dev_sender.send(Dev::Reload).is_err() {
                        break;
                    }
                    wake.wake_up();
                }
                // The dev server is gone; so are we.
                let _ = dev_sender.send(Dev::Quit);
                wake.wake_up();
            })
            .map_err(|e| e.to_string())?;
    }
    let (proxy, receiver) = BlitzShellProxy::new(event_loop.create_proxy());

    let doc = SvelteDocument::new(&js, &css, DocumentConfig::default())?;

    let (width, height) = manifest.size();
    let title = manifest
        .title
        .or(manifest.name)
        .unwrap_or_else(|| "svelte-blitz".into());
    let mut attributes = WindowAttributes::default()
        .with_title(title)
        .with_surface_size(LogicalSize::new(width, height));

    let mut dock_icon = None;
    if let Some(icon) = manifest.icon {
        // Title bar and taskbar on Windows and X11. macOS has no per-window icon
        // (see `set_dock_icon`), and Wayland takes it from a .desktop file.
        let rgba = RgbaIcon::new(icon.rgba, icon.width, icon.height).map_err(|e| e.to_string())?;
        attributes = attributes.with_window_icon(Some(rgba.into()));
        dock_icon = Some(icon.png);
    }

    let mut application = BlitzApplication::new(proxy, receiver);
    application.add_window(WindowConfig::with_attributes(
        Box::new(doc),
        VelloHybridWindowRenderer::new(),
        attributes,
    ));

    event_loop
        .run_app(Host {
            application,
            dock_icon,
            reload,
            dev_events,
        })
        .map_err(|e| e.to_string())
}

/// `BlitzApplication`, plus what it has no hook for: work that has to wait
/// until the platform application has launched, and dev reloads. Everything
/// else forwards.
struct Host {
    application: BlitzApplication<VelloHybridWindowRenderer>,
    dock_icon: Option<Vec<u8>>,
    reload: Option<Reload>,
    dev_events: Receiver<Dev>,
}

impl Host {
    /// Replace the document with a freshly loaded one. The window, its size and
    /// the viewport scroll position stay; component state starts over.
    fn reload(&mut self) {
        let Some(load) = &self.reload else { return };
        let started = Instant::now();
        let doc = load()
            .and_then(|app| SvelteDocument::new(&app.js, &app.css, DocumentConfig::default()));
        match doc {
            Ok(doc) => {
                if let Some(view) = self.application.windows.values_mut().next() {
                    view.replace_document(Box::new(doc), true);
                    eprintln!("[host] reloaded in {:.0?}", started.elapsed());
                }
            }
            // Keep showing the last build that worked; the next save tries again.
            Err(error) => eprintln!("[host] reload failed, keeping the previous build: {error}"),
        }
    }
}

impl ApplicationHandler for Host {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        // winit calls this for the first time right after launch, once it has made
        // the process a regular (Dock-visible) application. Earlier is too early.
        if let Some(png) = self.dock_icon.take() {
            set_dock_icon(&png);
        }
        self.application.can_create_surfaces(event_loop);
    }

    fn destroy_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.application.destroy_surfaces(event_loop);
    }

    fn resumed(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.application.resumed(event_loop);
    }

    fn suspended(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.application.suspended(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.application.window_event(event_loop, window_id, event);
    }

    fn proxy_wake_up(&mut self, event_loop: &dyn ActiveEventLoop) {
        // Several rebuilds may have queued up while one reload ran: do it once.
        let mut reload = false;
        for event in self.dev_events.try_iter() {
            match event {
                Dev::Reload => reload = true,
                Dev::Quit => {
                    // Drop the window before leaving the loop, as blitz-shell does
                    // on close (winit#4135).
                    self.application.windows.clear();
                    event_loop.exit();
                    return;
                }
            }
        }
        if reload {
            self.reload();
        }
        self.application.proxy_wake_up(event_loop);
    }

    fn macos_handler(&mut self) -> Option<&mut dyn ApplicationHandlerExtMacOS> {
        self.application.macos_handler()
    }
}

/// Give the process the app's name. macOS names an app after its bundle, and an
/// unbundled binary has none, so the menu bar would say `svelte-blitz-host`.
#[cfg(target_os = "macos")]
fn set_app_name(name: &str) {
    use objc2_foundation::{NSProcessInfo, NSString};

    NSProcessInfo::processInfo().setProcessName(&NSString::from_str(name));
}

#[cfg(not(target_os = "macos"))]
fn set_app_name(_name: &str) {}

/// Set the Dock icon. An unbundled binary has no `Info.plist` to name one, so
/// this is the only way to get anything but the generic executable icon.
#[cfg(target_os = "macos")]
fn set_dock_icon(png: &[u8]) {
    use objc2::{AnyThread, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::{NSBundle, NSData};

    // A bundle names its icon in Info.plist, and macOS then treats it like
    // every other app's. Overriding that at runtime would only undo it.
    if NSBundle::mainBundle().bundleIdentifier().is_some() {
        return;
    }
    let Some(main_thread) = MainThreadMarker::new() else {
        return;
    };
    // The bytes were already decoded once by `decode_icon`, so AppKit rejecting
    // them is not expected; if it does, keep the default icon.
    let Some(image) = NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(png)) else {
        eprintln!("[host] AppKit could not read the app icon");
        return;
    };
    // SAFETY: the image is a valid, non-nil NSImage and we are on the main thread.
    unsafe { NSApplication::sharedApplication(main_thread).setApplicationIconImage(Some(&image)) };
}

#[cfg(not(target_os = "macos"))]
fn set_dock_icon(_png: &[u8]) {}
