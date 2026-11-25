//! [`SvelteDocument`]: a Blitz document whose tree is driven by a Svelte app.

use std::cell::{Cell, Ref, RefCell, RefMut};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::task::Context as TaskContext;
use std::task::Waker;
use std::time::{Duration, Instant};

use blitz_dom::{
    BaseDocument, DocGuard, DocGuardMut, Document, DocumentConfig, EventDriver, EventHandler,
    LocalName, NodeId, QualName, local_name, ns,
};
use blitz_traits::events::{
    BlitzKeyEvent, BlitzPointerEvent, BlitzPointerId, DomEvent, DomEventData, EventState,
    MouseEventButton, MouseEventButtons, PointerCoords, UiEvent,
};

use keyboard_types::{Key, NamedKey};

use crate::api::{Api, Work, denied};
use crate::app::App;
use crate::hud::{AppNumbers, Hud};
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
    /// Ops that can change what is on screen (see [`ApplyStats`]).
    pub visible: usize,
    pub created: usize,
    pub dropped: usize,
    pub detached: usize,
    /// Boxes whose scroll position changed, and that the app was told about.
    pub scrolled: usize,
    /// The performance overlay changed (`hud.rs`). Not the app's doing.
    pub hud: bool,
    /// The app waits for a frame (`requestAnimationFrame`), and the window
    /// is on screen to draw one.
    pub frame: bool,
}

impl TurnStats {
    fn add(&mut self, stats: ApplyStats) {
        self.batches += 1;
        self.ops += stats.ops;
        self.visible += stats.visible;
        self.created += stats.created;
        self.dropped += stats.dropped;
        self.detached += stats.detached;
    }

    /// The page may look different now, or the app waits for a frame: the
    /// window has to draw one.
    pub fn changed(&self) -> bool {
        self.visible > 0 || self.hud || self.frame
    }
}

/// The Blitz document and what the app has registered in it.
///
/// Behind a `RefCell` because two parties change it: the document's own turn
/// (`SvelteDocument::settle`), and a measurement made by a running script,
/// which has to apply what the script wrote and lay out before it can answer
/// (`SvelteDocument::create`, "forced layout"). The cell is what keeps the
/// second from happening while Blitz is dispatching an event and holds the
/// document itself.
struct Page {
    doc: BaseDocument,
    nodes: NodeMap,
    /// Event types that have at least one listener. Everything else never enters JS.
    listening: HashSet<String>,
    /// The animation clock, in seconds, when there is no window: tests and
    /// screenshots move it with `wait`. In a window the shell supplies the time.
    headless_time: f64,
    /// Shown in a window, whose shell keeps the animation clock.
    windowed: bool,
    /// Batches a forced layout applied, not yet counted in a turn's stats.
    forced: Vec<ApplyStats>,
    /// Layouts done since the last trace line, and the time they took.
    layouts: (u32, std::time::Duration),
    /// Layouts done here since the start, for the performance overlay.
    layout_count: u64,
    /// When the page was last laid out here, on the animation clock, if
    /// nothing has touched the document since. A script measures many nodes
    /// in a row; only the first of them should pay for a layout.
    laid_out_at: Option<f64>,
    /// Boxes that scrolled since the app was last told (`report_scrolls`).
    scrolled: Vec<NodeId>,
}

impl Page {
    fn apply(&mut self, batch: &crate::js::Batch) -> ApplyStats {
        let stats = ops::apply_batch(&mut self.doc, &mut self.nodes, &mut self.listening, batch);
        if stats.visible > 0 {
            self.laid_out_at = None;
        }
        if !self.nodes.scrolls.is_empty() {
            // How far a box can scroll depends on what the batch put in it.
            if self.has_size() {
                self.layout_if_stale();
            }
            for request in std::mem::take(&mut self.nodes.scrolls) {
                crate::scroll::apply(&mut self.doc, request, &mut self.scrolled);
            }
        }
        stats
    }

    /// Whether the last layout still describes the page. While something is
    /// animating that is only true for a moment: a few milliseconds later
    /// boxes have moved.
    fn layout_is_current(&self) -> bool {
        const SAME_MOMENT: f64 = 0.004;
        self.laid_out_at
            .is_some_and(|at| !self.doc.is_animating() || (self.time() - at).abs() < SAME_MOMENT)
    }

    /// Lay the page out unless the last layout is still good.
    fn layout_if_stale(&mut self) {
        if !self.layout_is_current() {
            self.layout();
        }
    }

    /// The time on the clock the animations run on.
    fn time(&self) -> f64 {
        if self.windowed {
            animation_clock().elapsed().as_secs_f64()
        } else {
            self.headless_time
        }
    }

    /// Lay the page out, as of now on that clock.
    fn layout(&mut self) {
        let time = self.time();
        let began = std::time::Instant::now();
        self.doc.resolve(time);
        self.laid_out_at = Some(time);
        self.layouts.0 += 1;
        self.layouts.1 += began.elapsed();
        self.layout_count += 1;
    }

    /// For how many more seconds a CSS animation or transition runs on a
    /// node: 0 when none does, infinite for one that repeats forever.
    fn animating(&self, js_id: u32) -> f64 {
        self.nodes
            .blitz(js_id)
            .and_then(|node| self.doc.animations_end(node))
            .map_or(0.0, |end| (end - self.time()).max(0.0))
    }

    /// Before the document has been given a size there is nothing to measure,
    /// and laying it out at 0 × 0 would only leave Blitz with transforms
    /// worked out for that size.
    fn has_size(&self) -> bool {
        self.doc.viewport().window_size != (0, 0)
    }

    /// Where a node is as of the last layout: `[x, y, width, height]` in
    /// viewport coordinates.
    fn box_of(&self, js_id: u32) -> [f64; 4] {
        self.nodes
            .blitz(js_id)
            // A node that is not on the page has no box, whatever its last layout says.
            .filter(|&node| {
                self.doc
                    .get_node(node)
                    .is_some_and(|node| node.flags.is_in_document())
            })
            .and_then(|node| crate::scroll::client_rect(&self.doc, node))
            .map_or([0.0; 4], |(x, y, width, height)| [x, y, width, height])
    }
}

pub struct SvelteDocument {
    page: Rc<RefCell<Page>>,
    js: JsEngine,
    /// The viewport size the app was last told, in CSS pixels.
    reported_viewport: (f64, f64),
    /// The box the app was last told for each node it measures.
    reported_boxes: HashMap<u32, [f64; 4]>,
    timers: TimerWaker,
    last_turn: TurnStats,
    /// Every turn so far, added up. For benchmarks that span several turns.
    totals: TurnStats,
    updater: Option<Arc<Updater>>,
    api: Arc<Api>,
    /// Answers to host calls, sent by the threads doing the work.
    answers: (Sender<Answer>, Receiver<Answer>),
    /// Wakes the event loop when an answer is ready. Set once there is a loop.
    waker: Arc<Mutex<Option<Waker>>>,
    /// Host calls wait in the queue while this is set: the document is going
    /// into a window that does not exist yet. See [`SvelteDocument::for_window`].
    hold_host_calls: bool,
    /// Nodes detached since the last collection the host asked for.
    detached_since_gc: usize,
    /// When to collect garbage (JS clock, ms). See [`GC_AFTER_DETACHED`].
    gc_at: Option<f64>,
    /// The performance overlay.
    hud: Hud,
    /// When the event that started this turn arrived, if one did.
    turn_began: Option<Instant>,
    /// The window is on screen. Hidden or minimised, it draws no frames, and
    /// `requestAnimationFrame` callbacks wait, as in a background tab.
    visible: bool,
}

/// Detached nodes are freed when JavaScript collects their objects, and an
/// app that has gone idle allocates nothing, so the engine's own collector
/// may never run: leaving a page of 5000 rows kept 200k nodes (about 900 MB)
/// for good. Past this many detached nodes the host collects once itself.
const GC_AFTER_DETACHED: usize = 10_000;
/// ... this long after, so the pause (about 200 ms for those 200k nodes in
/// QuickJS) does not land between a click and the page it shows. A one-shot
/// deadline, not a poll: nothing wakes the loop again until more is detached.
const GC_DELAY_MS: f64 = 1000.0;

/// The id of a host call and its outcome: JSON, or an error message.
type Answer = (u32, Result<String, String>);

/// How a host call gets done.
enum Plan {
    /// Already done, on the UI thread: here is the JSON.
    Now(String),
    /// On a thread; the answer arrives on a later turn.
    Later(Work),
}

/// How often one turn may lay the page out, report it, and apply the app's
/// answer. A popup settles in two or three; an app that answers every layout
/// with another change must not hold the frame forever.
const LAYOUT_ROUNDS: usize = 8;

/// The clock animations run on in a window. `window.rs` gives the shell the
/// same origin, so a layout done here and one done there agree on the time.
pub fn animation_clock() -> std::time::Instant {
    static ORIGIN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    *ORIGIN.get_or_init(std::time::Instant::now)
}

/// `SEPAL_TRACE` is set: report what the UI thread does (see `window.rs`).
pub(crate) fn trace_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("SEPAL_TRACE").is_some())
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

        let page = Rc::new(RefCell::new(Page {
            doc,
            nodes,
            listening: HashSet::new(),
            headless_time: 0.0,
            windowed: hold_host_calls,
            forced: Vec::new(),
            layouts: Default::default(),
            layout_count: 0,
            laid_out_at: None,
            scrolled: Vec::new(),
        }));

        let js = JsEngine::new()?;
        // Outside a window, time is the caller's to move (`advance_time`), for
        // scripts as for animations: one clock, or a script that waits for an
        // animation with a timer finishes at a different moment on every machine.
        if !hold_host_calls {
            js.set_time(0.0);
        }
        // A forced layout, as in a browser: a script that measures a node gets
        // its box as of now, which means applying what the script has written
        // and laying out first. The one place where a read crosses into the
        // host, and where the tree changes outside `settle`.
        let shared = page.clone();
        js.on_measure(move |id, take| {
            // Blitz is dispatching an event and holds the document. The script
            // gets no answer and falls back to the last layout it was told.
            let mut page = shared.try_borrow_mut().ok()?;
            if !page.has_size() {
                return None;
            }
            let mut restyle = false;
            if let Some(batch) = take() {
                // Scripts turn an animation off to measure what is under it.
                // Blitz stops the animation in that pass but still styles the
                // node with it, so such a node is styled once more.
                restyle = page
                    .nodes
                    .blitz(id)
                    .is_some_and(|node| page.doc.animations_end(node).is_some());
                let stats = page.apply(&batch);
                page.forced.push(stats);
            }
            page.layout_if_stale();
            if restyle
                && let Some(node) = page.nodes.blitz(id)
                && let Some(node) = page.doc.get_node_mut(node)
            {
                node.set_restyle_hint(blitz_dom::RestyleHint::RESTYLE_SELF);
                node.mark_ancestors_dirty();
                page.layout();
            }
            let [x, y, width, height] = page.box_of(id);
            let [.., scroll_width, scroll_height, client_width, client_height] = page
                .nodes
                .blitz(id)
                .and_then(|node| crate::scroll::metrics(&page.doc, node))
                .unwrap_or_default();
            Some([
                x,
                y,
                width,
                height,
                page.animating(id),
                client_width,
                client_height,
                scroll_width,
                scroll_height,
            ])
        })?;

        let mut this = Self {
            page,
            js,
            reported_viewport: (0.0, 0.0),
            reported_boxes: HashMap::new(),
            timers: TimerWaker::default(),
            last_turn: TurnStats::default(),
            totals: TurnStats::default(),
            updater,
            api: Arc::new(api),
            answers: mpsc::channel(),
            waker: Arc::default(),
            hold_host_calls,
            detached_since_gc: 0,
            gc_at: None,
            hud: Hud::default(),
            turn_began: None,
            visible: true,
        };

        this.js.eval(app_js)?;
        this.settle();
        Ok(this)
    }

    pub fn base(&self) -> Ref<'_, BaseDocument> {
        Ref::map(self.page.borrow(), |page| &page.doc)
    }

    pub fn base_mut(&mut self) -> RefMut<'_, BaseDocument> {
        RefMut::map(self.page.borrow_mut(), |page| {
            // Whoever asks for this may change anything.
            page.laid_out_at = None;
            &mut page.doc
        })
    }

    /// Lay the page out now. A window does this before it paints; tests and
    /// screenshots have to ask.
    pub(crate) fn resolve_layout(&mut self) {
        self.page.borrow_mut().layout();
    }

    /// Move the animation clock of a document that is not in a window.
    pub(crate) fn advance_time(&mut self, seconds: f64) {
        let mut page = self.page.borrow_mut();
        page.headless_time += seconds;
        self.js.set_time(page.headless_time * 1000.0);
    }

    pub fn js(&self) -> &JsEngine {
        &self.js
    }

    /// Number of JS-created nodes the host currently holds.
    pub fn live_nodes(&self) -> usize {
        self.page.borrow().nodes.len()
    }

    /// Stats for every turn so far, added up.
    pub fn totals(&self) -> TurnStats {
        self.totals
    }

    /// Stats for the most recent turn.
    pub fn last_turn(&self) -> TurnStats {
        self.last_turn
    }

    pub fn is_listening(&self, event: &str) -> bool {
        self.page.borrow().listening.contains(event)
    }

    /// Finish a turn: run due timers and queued microtasks, then apply whatever
    /// the app changed as a single batch. Returns what happened.
    ///
    /// This is the only place the tree is mutated after startup.
    pub fn settle(&mut self) -> TurnStats {
        self.turn(true)
    }

    /// The window is about to draw a frame. Run the callbacks the app queued
    /// with `requestAnimationFrame`, all with this frame's time, and apply
    /// what they changed, so that this frame paints it.
    ///
    /// Timers wait for the turn after the frame, as in a browser: SvelteKit
    /// follows a link from a `setTimeout(0)` queued in a frame callback, to
    /// let the page paint first.
    pub fn begin_frame(&mut self) -> TurnStats {
        if !self.visible || !self.js.frame_pending() {
            return TurnStats::default();
        }
        self.turn_began.get_or_insert_with(Instant::now);
        self.js.run_frame(self.js.now());
        self.turn(false)
    }

    /// The window was hidden or minimised, or shows again. Hidden, it draws
    /// no frames, so the app's frame callbacks wait for the first one after it shows.
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// One turn: what [`SvelteDocument::settle`] says, with the due timers
    /// run or left for the next turn.
    fn turn(&mut self, timers: bool) -> TurnStats {
        let began = self.turn_began.take().unwrap_or_else(Instant::now);
        let mut turn = TurnStats::default();

        self.js.run_jobs();
        // Host calls that finished since the last turn.
        while let Ok((id, result)) = self.answers.1.try_recv() {
            self.js.resolve_call(id, result);
        }
        if timers {
            self.js.run_timers();
            self.js.run_jobs();
        }

        self.apply_batches(&mut turn);

        // An app that measures the page is told the new layout now, before
        // the frame is drawn, and may change the page again in answer: a
        // popup moves next to its button once it knows where the button is.
        // Any op counts here, not only visible ones: a node that was just
        // marked for measuring is owed its first report.
        let mut moved = turn.ops > 0 || turn.scrolled > 0;
        for round in 0.. {
            if !moved || !self.measuring() {
                break;
            }
            if round == LAYOUT_ROUNDS {
                eprintln!(
                    "[host] the page was still changing after {LAYOUT_ROUNDS} layouts in one turn; the rest waits for the next frame"
                );
                break;
            }
            self.page.borrow_mut().layout_if_stale();
            if !self.report_layout() {
                break;
            }
            self.js.run_jobs();
            let before = turn.ops;
            self.apply_batches(&mut turn);
            moved = turn.ops != before;
        }

        self.collect_detached(&mut turn);

        self.hud.turn(began.elapsed(), turn.ops);
        turn.hud = self.update_hud();
        turn.frame = self.visible && self.js.frame_pending();

        // Read the deadline last: effects that ran above may have started or stopped timers.
        let now = self.js.now();
        let next = match (self.js.next_timer(), self.gc_at) {
            (Some(timer), Some(gc)) => Some(timer.min(gc)),
            (timer, gc) => timer.or(gc),
        };
        let mut wait = next.map(|at| at - now);
        // The overlay's refresh, only while it is shown. On the real clock,
        // where the JS one stands still outside a window.
        if let Some(due) = self.hud.due() {
            let hud = due.saturating_duration_since(Instant::now()).as_secs_f64() * 1000.0;
            wait = Some(wait.map_or(hud, |wait| wait.min(hud)));
        }
        self.timers.set_deadline(wait);

        if turn.visible > 0 && trace_enabled() {
            let (layouts, took) = std::mem::take(&mut self.page.borrow_mut().layouts);
            eprintln!(
                "[trace] the app changed the page: {} ops, {layouts} layouts in {:.1}ms",
                turn.ops,
                took.as_secs_f64() * 1000.0
            );
        }
        self.last_turn = turn;
        self.totals.batches += turn.batches;
        self.totals.ops += turn.ops;
        self.totals.visible += turn.visible;
        self.totals.created += turn.created;
        self.totals.dropped += turn.dropped;
        self.totals.detached += turn.detached;
        self.totals.scrolled += turn.scrolled;
        turn
    }

    /// Schedule a collection once many nodes were detached, and run it when due.
    fn collect_detached(&mut self, turn: &mut TurnStats) {
        let now = self.js.now();
        if self.gc_at.is_some_and(|at| now >= at) {
            self.gc_at = None;
            let detached = std::mem::take(&mut self.detached_since_gc);
            let started = std::time::Instant::now();
            self.js.collect_garbage();
            // Finalizers queue DROPs; apply them now.
            self.js.run_jobs();
            self.apply_batches(turn);
            if trace_enabled() {
                eprintln!(
                    "[trace] collected garbage after {detached} detached nodes: {:.1}ms, {} nodes left",
                    started.elapsed().as_secs_f64() * 1000.0,
                    self.live_nodes()
                );
            }
        }
        self.detached_since_gc += turn.detached;
        if self.detached_since_gc >= GC_AFTER_DETACHED && self.gc_at.is_none() {
            self.gc_at = Some(now + GC_DELAY_MS);
        }
    }

    /// Apply what the app wrote, and what it writes in the microtasks that
    /// follow, and in answer to being told what scrolled.
    fn apply_batches(&mut self, turn: &mut TurnStats) {
        let mut scroll_rounds = 0;
        loop {
            // Before the batch: a call that is refused on the spot rejects its
            // promise right here, and what that renders belongs in this turn.
            self.start_host_calls();
            // What a script's measurements made the host apply ahead of the turn.
            let forced = std::mem::take(&mut self.page.borrow_mut().forced);
            forced.into_iter().for_each(|stats| turn.add(stats));
            let Some(batch) = self.js.take_batch() else {
                if scroll_rounds < LAYOUT_ROUNDS && self.report_scrolls(turn) {
                    scroll_rounds += 1;
                    continue;
                }
                if !self.page.borrow().scrolled.is_empty() {
                    eprintln!(
                        "[host] the app was still scrolling after {LAYOUT_ROUNDS} scroll events in one turn; the rest waits for the next"
                    );
                }
                break;
            };
            let stats = self.page.borrow_mut().apply(&batch);
            turn.add(stats);
            self.js.run_jobs();
        }
    }

    /// Tell the app which boxes scrolled since it was last told, with a
    /// `scroll` event on each, as a browser does once per frame. The event
    /// carries the new position, which the renderer keeps for `scrollTop`,
    /// so it is sent whether or not anyone listens. Returns whether any was.
    fn report_scrolls(&mut self, turn: &mut TurnStats) -> bool {
        let events: Vec<(u32, [f64; 6])> = {
            let mut page = self.page.borrow_mut();
            let scrolled = std::mem::take(&mut page.scrolled);
            scrolled
                .into_iter()
                // The viewport (<html>) scrolls too, but it is not the app's.
                .filter_map(|node| {
                    Some((
                        page.nodes.js(node)?,
                        crate::scroll::metrics(&page.doc, node)?,
                    ))
                })
                .collect()
        };
        for &(id, metrics) in &events {
            self.js
                .dispatch(id, "scroll", false, &EventDetail::Scroll(metrics));
        }
        turn.scrolled += events.len();
        self.js.run_jobs();
        !events.is_empty()
    }

    /// The app measures the page, and there is a page to measure.
    fn measuring(&self) -> bool {
        let page = self.page.borrow();
        !page.nodes.measured().is_empty() && page.has_size()
    }

    /// Tell the app the viewport size and the boxes of the nodes it measures
    /// (`MEASURE` in `ops.rs`), if any of that changed since it was last told.
    /// Returns whether it was told anything.
    ///
    /// The layout must be up to date. JavaScript never asks the host for a
    /// measurement; this is the only way one reaches it.
    fn report_layout(&mut self) -> bool {
        let (viewport, changed) = {
            let page = self.page.borrow();
            let measured = page.nodes.measured();
            if measured.is_empty() || !page.has_size() {
                return false;
            }
            let viewport = page.doc.viewport();
            let scale = viewport.scale_f64();
            let (width, height) = viewport.window_size;
            let viewport = (width as f64 / scale, height as f64 / scale);

            let mut changed = Vec::new();
            for &id in measured {
                let rect = page.box_of(id);
                if self.reported_boxes.get(&id) != Some(&rect) {
                    self.reported_boxes.insert(id, rect);
                    changed.push(id as f64);
                    changed.extend(rect);
                }
            }
            if changed.is_empty() && viewport == self.reported_viewport {
                return false;
            }
            self.reported_viewport = viewport;
            // Nodes that were freed since. Only worth a pass once they add up.
            if self.reported_boxes.len() > measured.len() * 2 {
                self.reported_boxes.retain(|id, _| measured.contains(id));
            }
            (viewport, changed)
        };
        if trace_enabled() {
            eprintln!(
                "[trace] the app was told the layout: {} boxes changed, window {} x {}",
                changed.len() / 5,
                viewport.0,
                viewport.1
            );
        }
        self.js.layout(viewport.0, viewport.1, changed);
        true
    }

    /// Call after the page was laid out for a reason the app did not cause:
    /// the window was resized, an animation moved on, an image arrived.
    /// Reports the new layout to an app that measures, and applies what it
    /// changes in answer. Returns whether it changed anything.
    pub fn after_layout(&mut self) -> bool {
        self.report_layout() && self.settle().changed()
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
                .name(format!("sepal-{}", call.name))
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
        let shell = self.page.borrow().doc.shell_provider.clone();
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
        let shell = self.page.borrow().doc.shell_provider.clone();
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

    /// Show or hide the performance overlay (`hud.rs`). The tree changes in
    /// the turn this ends with, as it does for the app.
    pub fn set_hud(&mut self, shown: bool) {
        self.hud.show(shown);
        self.settle();
    }

    pub fn hud_shown(&self) -> bool {
        self.hud.shown()
    }

    /// The window painted a frame, which held the UI thread this long.
    pub fn hud_frame(&mut self, took: Duration) {
        self.hud.frame(took);
    }

    fn update_hud(&mut self) -> bool {
        let live_nodes = self.live_nodes();
        let page = &mut *self.page.borrow_mut();
        let numbers = AppNumbers {
            live_nodes,
            layouts: page.layout_count,
        };
        let changed = self.hud.update(&mut page.doc, &numbers);
        if changed {
            page.laid_out_at = None;
        }
        changed
    }

    /// Run a garbage collection in JS and free the host nodes it released.
    pub fn collect_garbage(&mut self) -> TurnStats {
        self.js.collect_garbage();
        self.settle()
    }
}

impl Document for SvelteDocument {
    fn inner(&self) -> DocGuard<'_> {
        DocGuard::RefCell(self.base())
    }

    fn inner_mut(&mut self) -> DocGuardMut<'_> {
        DocGuardMut::RefCell(self.base_mut())
    }

    fn handle_ui_event(&mut self, event: UiEvent) {
        self.turn_began = Some(Instant::now());
        {
            // Handlers run synchronously while Blitz dispatches, which is what makes
            // `preventDefault()` work. That is safe because handlers never touch the
            // document: their mutations only land in the JS op buffer.
            // The document stays borrowed for the whole dispatch, which is
            // also what refuses a forced layout from inside a handler.
            let page = &mut *self.page.borrow_mut();
            // Input moves hover, focus and scroll positions.
            page.laid_out_at = None;
            let activates = activated_by(&page.doc, &event);
            let cancelled = Cell::new(false);
            let handler = JsEventHandler {
                js: &self.js,
                nodes: &page.nodes,
                listening: &page.listening,
                cancelled: &cancelled,
                scrolled: &mut page.scrolled,
            };
            let mut driver = EventDriver::new(&mut page.doc, handler);
            driver.handle_ui_event(event);
            // Enter and Space press the button that has the focus. Blitz moves
            // the focus with Tab but leaves this out.
            if let Some(click) = activates
                && !cancelled.get()
            {
                driver.handle_dom_event(click);
            }
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
    /// Set when a handler cancelled a `keydown`.
    cancelled: &'a Cell<bool>,
    /// Boxes Blitz scrolled. The app hears of them at the end of the turn.
    scrolled: &'a mut Vec<NodeId>,
}

/// The click a key press stands for, if it is Enter or Space on a focused
/// button, link or checkbox. To be sent after the `keydown`, unless a handler
/// cancelled that.
// ponytail: Space clicks on `keydown`; a browser waits for `keyup`.
fn activated_by(doc: &BaseDocument, event: &UiEvent) -> Option<DomEvent> {
    let UiEvent::KeyDown(key) = event else {
        return None;
    };
    let enter = key.key == Key::Named(NamedKey::Enter);
    let space = key.key == Key::Character(" ".into());
    if !(enter || space) || key.is_auto_repeating || !key.modifiers.is_empty() {
        return None;
    }
    let id = doc.get_focussed_node_id()?;
    let element = doc.get_node(id)?.element_data()?;
    if element.attr(local_name!("disabled")).is_some() {
        return None;
    }
    let kind = element.attr(local_name!("type")).unwrap_or("");
    let pressed = match &*element.name.local {
        "button" | "summary" => true,
        "a" => enter && element.attr(local_name!("href")).is_some(),
        "input" => match kind {
            "checkbox" | "radio" => space,
            "button" | "submit" | "reset" => true,
            _ => false,
        },
        _ => false,
    };
    if !pressed {
        return None;
    }
    // Where a pointer would have been: the middle of the element.
    let rect = doc.get_client_bounding_rect(id)?;
    let (x, y) = (
        (rect.x + rect.width / 2.0) as f32,
        (rect.y + rect.height / 2.0) as f32,
    );
    let click = BlitzPointerEvent {
        id: BlitzPointerId::Mouse,
        is_primary: true,
        coords: PointerCoords {
            page_x: x,
            page_y: y,
            screen_x: x,
            screen_y: y,
            client_x: x,
            client_y: y,
        },
        button: MouseEventButton::Main,
        buttons: MouseEventButtons::None,
        mods: Default::default(),
        details: Default::default(),
        element: Default::default(),
        active_pointers: Default::default(),
    };
    Some(DomEvent::new(id, DomEventData::Click(click)))
}

impl EventHandler for JsEventHandler<'_> {
    fn handle_event(
        &mut self,
        chain: &[NodeId],
        event: &mut DomEvent,
        _doc: &mut dyn Document,
        event_state: &mut EventState,
    ) {
        if let DomEventData::Scroll(_) = event.data {
            if !self.scrolled.contains(&event.target) {
                self.scrolled.push(event.target);
            }
            return;
        }
        let name = event.name();
        if !self.listening.contains(name) {
            return;
        }

        // The target may be a node Blitz created itself (an anonymous box, say).
        // Start from the nearest node the app knows about. Outside all of them
        // (on <html>) it is id 0, which only an app with a `document` listens to.
        let target = std::iter::once(&event.target)
            .chain(chain.iter())
            .find_map(|id| self.nodes.js(*id))
            .unwrap_or(0);

        let detail = event_detail(&event.data);
        let flags = self.js.dispatch(target, name, event.bubbles, &detail);

        // In a browser the click that ends a press is not a default action
        // of `pointerup`, so cancelling one does not cancel the other. In Blitz
        // it is, so for these two events the cancellation stops here.
        let ends_a_press = matches!(
            event.data,
            DomEventData::PointerUp(_) | DomEventData::MouseUp(_)
        );
        if flags & ops::DISPATCH_PREVENT_DEFAULT != 0 && !ends_a_press {
            event_state.prevent_default();
            if matches!(event.data, DomEventData::KeyDown(_)) {
                self.cancelled.set(true);
            }
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
