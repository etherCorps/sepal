//! [`SvelteDocument`]: a Blitz document whose tree is driven by a Svelte app.

use std::collections::HashSet;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::task::Context as TaskContext;
use std::task::Waker;

use blitz_dom::{
    BaseDocument, DocGuard, DocGuardMut, Document, DocumentConfig, EventDriver, EventHandler,
    LocalName, NodeId, QualName, ns,
};
use blitz_traits::events::{
    BlitzKeyEvent, BlitzPointerEvent, DomEvent, DomEventData, EventState, UiEvent,
};

use crate::api::{Api, Work, denied};
use crate::app::App;
use crate::js::{EventDetail, JsEngine, Mods};
use crate::net::AppResources;
use crate::ops::{self, ApplyStats, NodeMap};
use crate::timers::TimerWaker;
use crate::updater::Updater;

/// What one turn (one entry into JS and everything it caused) did.
#[derive(Default, Debug, Clone, Copy)]
pub struct TurnStats {
    pub batches: usize,
    pub ops: usize,
    pub created: usize,
    pub dropped: usize,
}

impl TurnStats {
    fn add(&mut self, stats: ApplyStats) {
        self.batches += 1;
        self.ops += stats.ops;
        self.created += stats.created;
        self.dropped += stats.dropped;
    }

    pub fn changed(&self) -> bool {
        self.ops > 0
    }
}

pub struct SvelteDocument {
    doc: BaseDocument,
    js: JsEngine,
    nodes: NodeMap,
    /// Event types that have at least one listener. Everything else never enters JS.
    listening: HashSet<String>,
    timers: TimerWaker,
    last_turn: TurnStats,
    updater: Option<Arc<Updater>>,
    api: Arc<Api>,
    /// Answers to host calls, sent by the threads doing the work.
    answers: (Sender<Answer>, Receiver<Answer>),
    /// Wakes the event loop when an answer is ready. Set once there is a loop.
    waker: Arc<Mutex<Option<Waker>>>,
    /// Host calls wait in the queue while this is set: the document is going
    /// into a window that does not exist yet. See [`SvelteDocument::for_window`].
    hold_host_calls: bool,
}

/// The id of a host call and its outcome: JSON, or an error message.
type Answer = (u32, Result<String, String>);

/// How a host call gets done.
enum Plan {
    /// Already done, on the UI thread: here is the JSON.
    Now(String),
    /// On a thread; the answer arrives on a later turn.
    Later(Work),
}

fn html(name: &str) -> QualName {
    QualName::new(None, ns!(html), LocalName::from(name))
}

impl SvelteDocument {
    /// Create the document for an app, able to load the app's own files and
    /// `data:` URLs. This is what screenshots and tests use.
    pub fn for_app(app: &App) -> Result<Self, String> {
        Self::for_app_in(app, false)
    }

    /// Like [`for_app`](Self::for_app), for a document that is about to be
    /// shown in a window.
    ///
    /// An app may call the host while it starts up (set the title, show a
    /// dialog). At that point there is no window and the event loop is not
    /// running, so those calls are kept until the window polls the document
    /// for the first time, and start then.
    pub fn for_window(app: &App) -> Result<Self, String> {
        Self::for_app_in(app, true)
    }

    fn for_app_in(app: &App, hold_host_calls: bool) -> Result<Self, String> {
        let resources = AppResources::new(app.dir.as_deref());
        let config = DocumentConfig {
            base_url: resources.base_url(),
            net_provider: Some(Arc::new(resources)),
            ..Default::default()
        };
        let updater = app.manifest.updater.clone().map(|config| {
            let version = app.manifest.version.as_deref().unwrap_or("0.0.0");
            Arc::new(Updater::new(config, version, app.bundle.clone()))
        });
        let api = Api::new(app.manifest.permissions.clone(), app.manifest.app_id());
        Self::create(&app.js, &app.css, config, updater, api, hold_host_calls)
    }

    /// Create the document, load the stylesheet, then evaluate the app bundle,
    /// which mounts the app into `<body>`.
    pub fn new(app_js: &str, app_css: &str, config: DocumentConfig) -> Result<Self, String> {
        // No manifest, so no permissions: every desktop call is refused.
        Self::create(
            app_js,
            app_css,
            config,
            None,
            Api::new(Default::default(), None),
            false,
        )
    }

    fn create(
        app_js: &str,
        app_css: &str,
        config: DocumentConfig,
        updater: Option<Arc<Updater>>,
        api: Api,
        hold_host_calls: bool,
    ) -> Result<Self, String> {
        let mut doc = BaseDocument::new(config);

        // <html><head><style>…</style></head><body></body></html>
        let body = {
            let mut m = doc.mutate();
            let root = m.doc.root_node().id;
            let html_el = m.create_element(html("html"), Vec::new());
            let head = m.create_element(html("head"), Vec::new());
            let body = m.create_element(html("body"), Vec::new());
            m.append_children(root, &[html_el]);
            m.append_children(html_el, &[head, body]);
            if !app_css.is_empty() {
                let style = m.create_element(html("style"), Vec::new());
                let css = m.create_text_node(app_css);
                m.append_children(style, &[css]);
                m.append_children(head, &[style]);
            }
            body
        };

        let mut nodes = NodeMap::default();
        nodes.insert(ops::ROOT_ID, body);

        let js = JsEngine::new()?;
        let mut this = Self {
            doc,
            js,
            nodes,
            listening: HashSet::new(),
            timers: TimerWaker::default(),
            last_turn: TurnStats::default(),
            updater,
            api: Arc::new(api),
            answers: mpsc::channel(),
            waker: Arc::default(),
            hold_host_calls,
        };

        this.js.eval(app_js)?;
        this.settle();
        Ok(this)
    }

    pub fn base(&self) -> &BaseDocument {
        &self.doc
    }

    pub fn base_mut(&mut self) -> &mut BaseDocument {
        &mut self.doc
    }

    pub fn js(&self) -> &JsEngine {
        &self.js
    }

    /// Number of JS-created nodes the host currently holds.
    pub fn live_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Stats for the most recent turn.
    pub fn last_turn(&self) -> TurnStats {
        self.last_turn
    }

    pub fn is_listening(&self, event: &str) -> bool {
        self.listening.contains(event)
    }

    /// Finish a turn: run due timers and queued microtasks, then apply whatever
    /// the app changed as a single batch. Returns what happened.
    ///
    /// This is the only place the tree is mutated after startup.
    pub fn settle(&mut self) -> TurnStats {
        let mut turn = TurnStats::default();

        self.js.run_jobs();
        // Host calls that finished since the last turn.
        while let Ok((id, result)) = self.answers.1.try_recv() {
            self.js.resolve_call(id, result);
        }
        self.js.run_timers();
        self.js.run_jobs();

        loop {
            // Before the batch: a call that is refused on the spot rejects its
            // promise right here, and what that renders belongs in this turn.
            self.start_host_calls();
            let Some(batch) = self.js.take_batch() else {
                break;
            };
            let stats =
                ops::apply_batch(&mut self.doc, &mut self.nodes, &mut self.listening, &batch);
            turn.add(stats);
            self.js.run_jobs();
        }

        // Read the deadline last: effects that ran above may have started or stopped timers.
        let now = self.js.now();
        self.timers
            .set_deadline(self.js.next_timer().map(|at| at - now));

        self.last_turn = turn;
        turn
    }

    /// Do, or start, what JS asked for through `__sb_host.call`. Slow work
    /// goes to a thread, which sends its answer back and wakes the event
    /// loop; the answer is delivered at the start of the next turn.
    fn start_host_calls(&mut self) {
        if self.hold_host_calls {
            return;
        }
        for call in self.js.take_calls() {
            let work = match self.host_work(&call.name, &call.args) {
                Ok(Plan::Later(work)) => work,
                Ok(Plan::Now(json)) => {
                    self.js.resolve_call(call.id, Ok(json));
                    continue;
                }
                Err(refusal) => {
                    self.js.resolve_call(call.id, Err(refusal));
                    continue;
                }
            };
            let (answers, waker) = (self.answers.0.clone(), self.waker.clone());
            let spawned = std::thread::Builder::new()
                .name(format!("svelte-blitz-{}", call.name))
                .spawn(move || {
                    // Whatever happens in there, the promise has to settle.
                    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work))
                        .unwrap_or_else(|_| Err("the host failed while handling this call".into()));
                    let _ = answers.send((call.id, outcome));
                    if let Some(waker) = &*waker.lock().unwrap() {
                        waker.wake_by_ref();
                    }
                });
            if let Err(error) = spawned {
                self.js
                    .resolve_call(call.id, Err(format!("cannot start {}: {error}", call.name)));
            }
        }
    }

    /// What a host call does, or why it is refused.
    fn host_work(&self, name: &str, args: &str) -> Result<Plan, String> {
        let (group, action) = name.split_once('.').unwrap_or((name, ""));
        match group {
            "updater" => self.updater_work(name, action).map(Plan::Later),
            // These need the window and are quick, so they run right here.
            "window" => self.window_call(name, action, args).map(Plan::Now),
            "clipboard" => self.clipboard_call(name, action, args).map(Plan::Now),
            _ => self.api.call(name, args).map(Plan::Later),
        }
    }

    fn updater_work(&self, name: &str, action: &str) -> Result<Work, String> {
        let updater = self
            .updater
            .clone()
            .ok_or("this app has no `updater` in its manifest")?;
        Ok(match action {
            "check" => Box::new(move || {
                let update = updater.check()?;
                Ok(match update {
                    None => "null".to_owned(),
                    Some(update) => serde_json::json!({
                        "version": update.version,
                        "current_version": updater.current_version(),
                        "notes": update.notes,
                        "date": update.date,
                    })
                    .to_string(),
                })
            }),
            "download" => Box::new(move || updater.download().map(|()| "null".to_owned())),
            "install" => Box::new(move || {
                let bundle = updater.install()?;
                crate::updater::relaunch(&bundle)?;
                // The new version is starting; this one is done.
                std::process::exit(0)
            }),
            _ => return Err(format!("this host has no call named {name}")),
        })
    }

    fn window_call(&self, name: &str, action: &str, args: &str) -> Result<String, String> {
        if !self.api.permissions.window {
            return Err(denied("changing the window", "window"));
        }
        let args: serde_json::Value = serde_json::from_str(args).unwrap_or_default();
        let shell = &self.doc.shell_provider;
        match action {
            "set_title" => shell.set_window_title(args["title"].as_str().unwrap_or("").to_owned()),
            "minimize" => shell.set_window_minimized(true),
            "set_maximized" => {
                shell.set_window_maximized(args["maximized"].as_bool().unwrap_or(true))
            }
            "is_maximized" => return Ok(shell.is_window_maximized().to_string()),
            "set_decorations" => {
                shell.set_window_decorations(args["decorations"].as_bool().unwrap_or(true))
            }
            "start_dragging" => shell.drag_window(),
            "close" => shell.request_window_close(),
            _ => return Err(format!("this host has no call named {name}")),
        }
        Ok("null".to_owned())
    }

    fn clipboard_call(&self, name: &str, action: &str, args: &str) -> Result<String, String> {
        if !self.api.permissions.clipboard {
            return Err(denied("using the clipboard", "clipboard"));
        }
        let args: serde_json::Value = serde_json::from_str(args).unwrap_or_default();
        let shell = &self.doc.shell_provider;
        match action {
            "read_text" => shell
                .get_clipboard_text()
                .map(|text| serde_json::json!(text).to_string())
                .map_err(|_| "the clipboard is not available, or holds no text".to_owned()),
            "write_text" => shell
                .set_clipboard_text(args["text"].as_str().unwrap_or("").to_owned())
                .map(|()| "null".to_owned())
                .map_err(|_| "the clipboard is not available".to_owned()),
            _ => Err(format!("this host has no call named {name}")),
        }
    }

    /// Run a garbage collection in JS and free the host nodes it released.
    pub fn collect_garbage(&mut self) -> TurnStats {
        self.js.collect_garbage();
        self.settle()
    }
}

impl Document for SvelteDocument {
    fn inner(&self) -> DocGuard<'_> {
        DocGuard::Ref(&self.doc)
    }

    fn inner_mut(&mut self) -> DocGuardMut<'_> {
        DocGuardMut::Ref(&mut self.doc)
    }

    fn handle_ui_event(&mut self, event: UiEvent) {
        {
            // Handlers run synchronously while Blitz dispatches, which is what makes
            // `preventDefault()` work. That is safe because handlers never touch the
            // document: their mutations only land in the JS op buffer.
            let handler = JsEventHandler {
                js: &self.js,
                nodes: &self.nodes,
                listening: &self.listening,
            };
            let mut driver = EventDriver::new(&mut self.doc, handler);
            driver.handle_ui_event(event);
        }
        self.settle();
    }

    fn poll(&mut self, task_context: Option<TaskContext>) -> bool {
        if let Some(cx) = task_context {
            self.timers.set_waker(cx.waker());
            *self.waker.lock().unwrap() = Some(cx.waker().clone());
            // There is a window now, and its event loop is running.
            self.hold_host_calls = false;
        }
        self.settle().changed()
    }
}

struct JsEventHandler<'a> {
    js: &'a JsEngine,
    nodes: &'a NodeMap,
    listening: &'a HashSet<String>,
}

impl EventHandler for JsEventHandler<'_> {
    fn handle_event(
        &mut self,
        chain: &[NodeId],
        event: &mut DomEvent,
        _doc: &mut dyn Document,
        event_state: &mut EventState,
    ) {
        let name = event.name();
        if !self.listening.contains(name) {
            return;
        }

        // The target may be a node Blitz created itself (an anonymous box, say).
        // Start from the nearest node the app knows about.
        let Some(target) = std::iter::once(&event.target)
            .chain(chain.iter())
            .find_map(|id| self.nodes.js(*id))
        else {
            return;
        };

        let detail = event_detail(&event.data);
        let flags = self.js.dispatch(target, name, event.bubbles, &detail);

        if flags & ops::DISPATCH_PREVENT_DEFAULT != 0 {
            event_state.prevent_default();
        }
        if flags & ops::DISPATCH_STOP_PROPAGATION != 0 {
            event_state.stop_propagation();
        }
    }
}

fn pointer_detail(event: &BlitzPointerEvent) -> EventDetail {
    EventDetail::Pointer {
        client_x: event.coords.client_x,
        client_y: event.coords.client_y,
        page_x: event.coords.page_x,
        page_y: event.coords.page_y,
        offset_x: event.element.x,
        offset_y: event.element.y,
        button: event.button as i32,
        buttons: event.buttons.bits() as u32,
        mods: mods(event.mods),
    }
}

fn key_detail(event: &BlitzKeyEvent) -> EventDetail {
    EventDetail::Key {
        key: event.key.to_string(),
        code: event.code.to_string(),
        repeat: event.is_auto_repeating,
        mods: mods(event.modifiers),
    }
}

fn mods(modifiers: keyboard_types::Modifiers) -> Mods {
    Mods {
        alt: modifiers.alt(),
        ctrl: modifiers.ctrl(),
        meta: modifiers.meta(),
        shift: modifiers.shift(),
    }
}

fn event_detail(data: &DomEventData) -> EventDetail {
    use DomEventData::*;
    match data {
        PointerMove(e) | PointerDown(e) | PointerUp(e) | PointerCancel(e) | PointerEnter(e)
        | PointerLeave(e) | PointerOver(e) | PointerOut(e) | MouseMove(e) | MouseDown(e)
        | MouseUp(e) | MouseEnter(e) | MouseLeave(e) | MouseOver(e) | MouseOut(e)
        | TouchStart(e) | TouchMove(e) | TouchEnd(e) | TouchCancel(e) | Click(e)
        | ContextMenu(e) | DoubleClick(e) => pointer_detail(e),
        KeyPress(e) | KeyDown(e) | KeyUp(e) => key_detail(e),
        Input(e) => EventDetail::Input {
            value: e.value.clone(),
        },
        _ => EventDetail::None,
    }
}
