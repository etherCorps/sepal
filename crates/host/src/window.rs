//! Open a native window for a [`SvelteDocument`].
//!
//! The event loop comes from `blitz-shell` and runs with `ControlFlow::Wait`:
//! the process sleeps in the OS until there is input, a timer fires, or a
//! redraw was requested. Nothing is painted unless something changed.
//!
//! In dev (`--watch`) the host also reads lines from stdin, one per rebuild,
//! and swaps in a freshly loaded document without closing the window.
//!
//! Two more lines are understood on stdin, for driving a real window from a
//! script: `move <x> <y>` and `click <x> <y>`, in logical pixels. They enter
//! through the same function the OS's events do. `hud` shows or hides the
//! performance overlay, as Cmd+Option+P (Ctrl+Alt+P elsewhere) does.

use std::io::BufRead;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use anyrender_vello_hybrid::VelloHybridWindowRenderer;
use blitz_shell::{BlitzApplication, BlitzShellProxy, WindowConfig, create_default_event_loop};
use winit::application::ApplicationHandler;
use winit::application::macos::ApplicationHandlerExtMacOS;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::icon::RgbaIcon;
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::{WindowAttributes, WindowId};

use crate::SvelteDocument;
use crate::app::App;

/// Loads the app again from wherever it came from. Present in dev only.
pub type Reload = Box<dyn Fn() -> Result<App, String>>;

enum Dev {
    Reload,
    Quit,
    /// Move the pointer to a point in logical pixels, and optionally click there.
    Pointer {
        x: f64,
        y: f64,
        click: bool,
    },
    /// Press these keys, each named as `KeyboardEvent.key` names it.
    Keys(Vec<String>),
    /// Show or hide the performance overlay.
    Hud,
}

impl Dev {
    /// What a line on stdin asks for. Anything unrecognised is a reload, which
    /// is what the Vite preset sends.
    fn parse(line: &str) -> Self {
        if line.trim() == "hud" {
            return Dev::Hud;
        }
        // `key Escape`, `key ArrowDown`; `type some text` is a key per character.
        if let Some(name) = line.strip_prefix("key ") {
            return Dev::Keys(vec![name.trim().to_owned()]);
        }
        if let Some(text) = line.strip_prefix("type ") {
            return Dev::Keys(text.chars().map(String::from).collect());
        }
        let mut words = line.split_whitespace();
        let click = match words.next() {
            Some("click") => true,
            Some("move") => false,
            _ => return Dev::Reload,
        };
        match (
            words.next().and_then(|x| x.parse().ok()),
            words.next().and_then(|y| y.parse().ok()),
        ) {
            (Some(x), Some(y)) => Dev::Pointer { x, y, click },
            _ => Dev::Reload,
        }
    }
}

/// Run an app until its window is closed. With `reload`, also follow stdin:
/// every line reloads the app, end of input quits.
pub fn run(app: App, reload: Option<Reload>) -> Result<(), String> {
    // The renderer draws each glyph once and reuses it (a patch to the vendored
    // anyrender_vello_hybrid). If text ever looks wrong on some GPU, this turns
    // that off and tells us whether the atlas is to blame.
    if std::env::var_os("SVELTE_BLITZ_GLYPH_CACHE").is_some_and(|value| value == "0") {
        anyrender_vello_hybrid::set_glyph_cache(false);
    }
    let mut doc = SvelteDocument::for_window(&app)?;
    if std::env::var_os("SVELTE_BLITZ_HUD").is_some_and(|value| value == "1") {
        doc.set_hud(true);
    }
    let manifest = app.manifest;
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
                    let Ok(line) = line else { break };
                    if dev_sender
                        .send((Instant::now(), Dev::parse(&line)))
                        .is_err()
                    {
                        break;
                    }
                    wake.wake_up();
                }
                // The dev server is gone; so are we.
                let _ = dev_sender.send((Instant::now(), Dev::Quit));
                wake.wake_up();
            })
            .map_err(|e| e.to_string())?;
    }
    let (proxy, receiver) = BlitzShellProxy::new(event_loop.create_proxy());

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
            modifiers: ModifiersState::empty(),
            skipped_frame: None,
            input_since_frame: false,
            redraw_at: None,
            trace: std::env::var_os("SVELTE_BLITZ_TRACE").map(|_| Trace {
                started: Instant::now(),
                first_frame: false,
            }),
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
    /// What stdin asked for, and when it arrived.
    dev_events: Receiver<(Instant, Dev)>,
    /// Held down now, for the overlay's shortcut.
    modifiers: ModifiersState,
    /// When the renderer last skipped a frame (nothing on screen changed), whether
    /// input came in since, and a redraw put off until the next 60 Hz frame.
    /// See `defer_redraw`.
    skipped_frame: Option<Instant>,
    input_since_frame: bool,
    redraw_at: Option<(WindowId, Instant)>,
    /// Set by `SVELTE_BLITZ_TRACE`: report where the UI thread spends its time.
    trace: Option<Trace>,
}

/// Timing of the UI thread, written to stderr as `[trace] +<since launch> …`.
/// Input and the first frame are always reported; anything else only when it
/// held the thread for longer than a frame should take.
struct Trace {
    started: Instant,
    first_frame: bool,
}

impl Trace {
    /// A frame at 120 Hz. Work shorter than this cannot be felt.
    const SLOW: std::time::Duration = std::time::Duration::from_millis(8);

    fn note(&self, what: &str) {
        let at = self.started.elapsed().as_secs_f64();
        eprintln!("[trace] +{at:.3}s {what}");
    }

    fn report(&mut self, what: &str, began: Instant, always: bool) {
        let took = began.elapsed();
        if always || took >= Self::SLOW {
            let at = began.duration_since(self.started).as_secs_f64();
            eprintln!(
                "[trace] +{at:.3}s {what} took {:.1}ms",
                took.as_secs_f64() * 1000.0
            );
        }
    }
}

impl Host {
    /// Feed pointer events to the window as if the OS had sent them.
    fn pointer(&mut self, event_loop: &dyn ActiveEventLoop, x: f64, y: f64, click: bool) {
        use winit::event::{ButtonSource, ElementState, MouseButton, PointerSource};

        let Some((&window_id, view)) = self.application.windows.iter().next() else {
            return;
        };
        let scale = view.window.scale_factor();
        let position = winit::dpi::PhysicalPosition::new(x * scale, y * scale);
        let moved = WindowEvent::PointerMoved {
            device_id: None,
            position,
            primary: true,
            source: PointerSource::Mouse,
        };
        self.window_event(event_loop, window_id, moved);
        if click {
            for state in [ElementState::Pressed, ElementState::Released] {
                let button = WindowEvent::PointerButton {
                    device_id: None,
                    state,
                    position,
                    primary: true,
                    button: ButtonSource::Mouse(MouseButton::Left),
                    is_macos_activation_click: false,
                };
                self.window_event(event_loop, window_id, button);
            }
        }
    }

    /// Press keys in the window's document. Unlike pointer events these do
    /// not pass through winit, which has no way to make a key event: they
    /// start where Blitz would hand one to the document.
    fn keys(&mut self, keys: &[String]) {
        let Some(view) = self.application.windows.values_mut().next() else {
            return;
        };
        let began = Instant::now();
        let doc = view.downcast_doc_mut::<SvelteDocument>();
        for key in keys {
            doc.press(key);
        }
        view.request_redraw();
        if let Some(trace) = &mut self.trace {
            trace.report(&format!("keys {keys:?}"), began, true);
        }
    }

    /// Show the performance overlay, or hide it.
    fn toggle_hud(&mut self) {
        let Some(view) = self.application.windows.values_mut().next() else {
            return;
        };
        let doc = view.downcast_doc_mut::<SvelteDocument>();
        doc.set_hud(!doc.hud_shown());
        view.request_redraw();
    }

    /// Cmd+Option+P on macOS, Ctrl+Alt+P elsewhere. By the key's place, not
    /// its letter: Option turns P into π.
    fn is_hud_shortcut(&self, key: &winit::event::KeyEvent) -> bool {
        let command = if cfg!(target_os = "macos") {
            self.modifiers.meta_key()
        } else {
            self.modifiers.control_key()
        };
        command && self.modifiers.alt_key() && key.physical_key == PhysicalKey::Code(KeyCode::KeyP)
    }

    /// Hand a window event to Blitz, timing it when tracing is on.
    fn forward(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(trace) = &mut self.trace else {
            return self.application.window_event(event_loop, window_id, event);
        };
        // The variant's name: "PointerButton", "RedrawRequested", …
        let name: String = format!("{event:?}")
            .chars()
            .take_while(|c| c.is_alphanumeric())
            .collect();
        // Input and repaints are always reported: what matters is that one follows the other.
        let input = matches!(
            event,
            WindowEvent::PointerButton { .. }
                | WindowEvent::KeyboardInput { .. }
                | WindowEvent::RedrawRequested
        );
        let first_frame = matches!(event, WindowEvent::RedrawRequested) && !trace.first_frame;
        let began = Instant::now();
        self.application.window_event(event_loop, window_id, event);
        let trace = self.trace.as_mut().unwrap();
        if first_frame {
            trace.first_frame = true;
            trace.report("first frame", began, true);
        } else {
            trace.report(&name, began, input);
        }
    }

    /// The renderer skips a frame in which nothing on screen changed, and
    /// presenting nothing does not wait for the display the way presenting a
    /// frame does. Something animating without changing pixels asks for the next
    /// frame right after each one, so it would spin. Such a redraw waits for the
    /// next 60 Hz frame instead, in the OS, not on this thread: input that comes
    /// meanwhile is handled at once, and the redraw it asks for is not put off.
    fn defer_redraw(&mut self, window_id: WindowId) -> bool {
        const FRAME: Duration = Duration::from_micros(16_667);
        let Some(skipped) = self.skipped_frame else {
            return false;
        };
        let due = skipped + FRAME;
        if self.input_since_frame || Instant::now() >= due {
            return false;
        }
        self.redraw_at = Some((window_id, due));
        true
    }

    /// Replace the document with a freshly loaded one. The window, its size and
    /// the viewport scroll position stay; component state starts over.
    fn reload(&mut self) {
        let Some(load) = &self.reload else { return };
        let started = Instant::now();
        let doc = load().and_then(|app| SvelteDocument::for_window(&app));
        match doc {
            Ok(mut doc) => {
                if let Some(view) = self.application.windows.values_mut().next() {
                    // The overlay stays up across a reload.
                    if view.downcast_doc_mut::<SvelteDocument>().hud_shown() {
                        doc.set_hud(true);
                    }
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
        // One clock for animations, whoever lays the page out: the shell before
        // it paints, or the document when the app measures (`SvelteDocument::settle`).
        for view in self.application.windows.values_mut() {
            view.animation_timer = Some(crate::document::animation_clock());
        }
        match &event {
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            // A frame drawn before the window was first on screen is lost, and the
            // redraw that follows would find nothing changed and leave it blank.
            WindowEvent::Occluded(false) => {
                if let Some(view) = self.application.windows.get_mut(&window_id) {
                    view.renderer.repaint_all();
                }
            }
            // The host's own shortcut: the app never sees it.
            WindowEvent::KeyboardInput { event: key, .. } if self.is_hud_shortcut(key) => {
                if key.state.is_pressed() && !key.repeat {
                    self.toggle_hud();
                }
                return;
            }
            _ => {}
        }
        let event = match self.application.windows.get(&window_id) {
            Some(view) => logical_wheel(event, view.window.scale_factor()),
            None => event,
        };
        if matches!(event, WindowEvent::RedrawRequested) {
            if self.defer_redraw(window_id) {
                return;
            }
            self.redraw_at = None;
            self.input_since_frame = false;
            let began = Instant::now();
            self.forward(event_loop, window_id, event);
            let took = began.elapsed();
            // The frame was laid out to be painted. If that moved something the
            // app measures (a resize, an animation), it hears about it now.
            if let Some(view) = self.application.windows.get_mut(&window_id) {
                self.skipped_frame = view.renderer.skipped_last_frame().then(Instant::now);
                let doc = view.downcast_doc_mut::<SvelteDocument>();
                doc.hud_frame(took);
                if doc.after_layout() {
                    view.request_redraw();
                }
            }
            return;
        }
        self.input_since_frame = true;
        self.forward(event_loop, window_id, event);
    }

    fn about_to_wait(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.application.about_to_wait(event_loop);
        // Two seconds after the last frame, the renderer lets go of the kept frame
        // and the depth buffer, the first time anything wakes the loop after that.
        // Never on a deadline of its own: an idle app is not woken for it
        // (invariant 7), so one left alone keeps them.
        for view in self.application.windows.values_mut() {
            if view.renderer.release_if_idle(Duration::from_secs(2)) {
                if let Some(trace) = &self.trace {
                    trace.note("released the kept frame and the depth buffer (no frame for 2 s)");
                }
            }
        }
        // A one-shot deadline, set again only by the next skipped frame: nothing
        // here wakes the loop on its own (invariant 7).
        let flow = match self.redraw_at {
            Some((_, due)) if Instant::now() < due => ControlFlow::WaitUntil(due),
            Some((window_id, _)) => {
                self.redraw_at = None;
                if let Some(view) = self.application.windows.get(&window_id) {
                    view.request_redraw();
                }
                ControlFlow::Wait
            }
            None => ControlFlow::Wait,
        };
        event_loop.set_control_flow(flow);
    }

    fn proxy_wake_up(&mut self, event_loop: &dyn ActiveEventLoop) {
        // Several rebuilds may have queued up while one reload ran: do it once.
        let mut reload = false;
        let events: Vec<(Instant, Dev)> = self.dev_events.try_iter().collect();
        if !events.is_empty() {
            self.input_since_frame = true;
        }
        for (arrived, event) in events {
            if let (Some(trace), Dev::Pointer { x, y, click }) = (&mut self.trace, &event) {
                let what = if *click { "click" } else { "move" };
                trace.note(&format!(
                    "stdin {what} {x} {y}, {:.1}ms after it arrived",
                    arrived.elapsed().as_secs_f64() * 1000.0
                ));
            }
            match event {
                Dev::Reload => reload = true,
                Dev::Pointer { x, y, click } => self.pointer(event_loop, x, y, click),
                Dev::Keys(keys) => self.keys(&keys),
                Dev::Hud => self.toggle_hud(),
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
        let began = Instant::now();
        self.application.proxy_wake_up(event_loop);
        if let Some(trace) = &mut self.trace {
            // Polls the document, and updates the accessibility tree when it changed.
            trace.report("poll", began, false);
        }
    }

    fn macos_handler(&mut self) -> Option<&mut dyn ApplicationHandlerExtMacOS> {
        self.application.macos_handler()
    }
}

/// blitz-shell 0.3.0-beta.4 hands winit's `PixelDelta` (a trackpad, a precise
/// wheel), which is in physical pixels, to Blitz as CSS pixels, so on a 2x
/// display everything scrolled twice as far as the fingers moved. Turn it into
/// logical pixels before it gets there. `LineDelta` has no unit to fix.
pub fn logical_wheel(event: WindowEvent, scale: f64) -> WindowEvent {
    use winit::event::MouseScrollDelta;

    match event {
        WindowEvent::MouseWheel {
            device_id,
            delta: MouseScrollDelta::PixelDelta(pixels),
            phase,
        } => WindowEvent::MouseWheel {
            device_id,
            delta: MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(
                pixels.x / scale,
                pixels.y / scale,
            )),
            phase,
        },
        event => event,
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
