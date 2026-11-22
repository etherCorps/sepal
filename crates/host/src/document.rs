//! [`SvelteDocument`]: a Blitz document whose tree is driven by a Svelte app.

use std::collections::HashSet;
use std::task::Context as TaskContext;

use blitz_dom::{
    BaseDocument, DocGuard, DocGuardMut, Document, DocumentConfig, EventDriver, EventHandler,
    LocalName, NodeId, QualName, ns,
};
use blitz_traits::events::{
    BlitzKeyEvent, BlitzPointerEvent, DomEvent, DomEventData, EventState, UiEvent,
};

use crate::js::{EventDetail, JsEngine, Mods};
use crate::ops::{self, ApplyStats, NodeMap};
use crate::timers::TimerWaker;

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
}

fn html(name: &str) -> QualName {
    QualName::new(None, ns!(html), LocalName::from(name))
}

impl SvelteDocument {
    /// Create the document, load the stylesheet, then evaluate the app bundle,
    /// which mounts the app into `<body>`.
    pub fn new(app_js: &str, app_css: &str, config: DocumentConfig) -> Result<Self, String> {
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
        self.js.run_timers();
        self.js.run_jobs();

        while let Some(batch) = self.js.take_batch() {
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
