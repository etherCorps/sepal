//! Applies op batches from the JS renderer to a Blitz document.
//!
//! Keep the opcodes in sync with `packages/sepal/src/protocol.js`.

use std::collections::{HashMap, HashSet};

use blitz_dom::{BaseDocument, LocalName, NodeId, QualName, ns};

use crate::js::Batch;
use crate::scroll;

pub const CREATE_ELEMENT: i32 = 1; // id, name
pub const CREATE_TEXT: i32 = 2; // id, text
pub const CREATE_COMMENT: i32 = 3; // id, text
pub const SET_TEXT: i32 = 4; // id, text
pub const SET_ATTR: i32 = 5; // id, name, value
pub const REMOVE_ATTR: i32 = 6; // id, name
pub const INSERT: i32 = 7; // parent, node, anchor (0 = append)
pub const REMOVE: i32 = 8; // node
pub const DROP: i32 = 9; // node
pub const LISTEN: i32 = 10; // type
pub const UNLISTEN: i32 = 11; // type
pub const MEASURE: i32 = 12; // node
pub const FOCUS: i32 = 13; // node (0 = none)
pub const SCROLL: i32 = 14; // node, left, top
pub const SCROLL_INTO_VIEW: i32 = 15; // node, block, inline (0 start, 1 center, 2 end, 3 nearest)

pub const DISPATCH_PREVENT_DEFAULT: u32 = 1;
pub const DISPATCH_STOP_PROPAGATION: u32 = 2;

/// Every SVG element name. The renderer API's `createElement(name)` carries no
/// namespace, so the name decides. Names that HTML also has (`a`, `title`,
/// `style`, `script`) stay HTML; those four are not listed.
///
/// `tests/icons.rs` fails if an element inside an `<svg>` is missing here.
const SVG_ELEMENTS: &[&str] = &[
    "animate",
    "animateMotion",
    "animateTransform",
    "circle",
    "clipPath",
    "defs",
    "desc",
    "ellipse",
    "feBlend",
    "feColorMatrix",
    "feComponentTransfer",
    "feComposite",
    "feConvolveMatrix",
    "feDiffuseLighting",
    "feDisplacementMap",
    "feDistantLight",
    "feDropShadow",
    "feFlood",
    "feFuncA",
    "feFuncB",
    "feFuncG",
    "feFuncR",
    "feGaussianBlur",
    "feImage",
    "feMerge",
    "feMergeNode",
    "feMorphology",
    "feOffset",
    "fePointLight",
    "feSpecularLighting",
    "feSpotLight",
    "feTile",
    "feTurbulence",
    "filter",
    "foreignObject",
    "g",
    "image",
    "line",
    "linearGradient",
    "marker",
    "mask",
    "metadata",
    "mpath",
    "path",
    "pattern",
    "polygon",
    "polyline",
    "radialGradient",
    "rect",
    "set",
    "stop",
    "svg",
    "switch",
    "symbol",
    "text",
    "textPath",
    "tspan",
    "use",
    "view",
];

/// Two-way mapping between ids allocated by the JS renderer and Blitz node ids.
#[derive(Default)]
pub struct NodeMap {
    to_blitz: HashMap<u32, NodeId>,
    to_js: HashMap<NodeId, u32>,
    /// JS ids of the nodes the app measures: it is told where they are after
    /// every layout. See `SvelteDocument::report_layout`.
    measured: HashSet<u32>,
    /// Scrolls the app asked for in this batch, applied after it, once the
    /// page is laid out (`Page::apply` in `document.rs`).
    pub scrolls: Vec<scroll::Request>,
    /// JS ids of the `<link>` elements the app put on the page since it was
    /// last told whether their stylesheets loaded (`report_links` in `document.rs`).
    pub links: Vec<u32>,
}

impl NodeMap {
    pub fn insert(&mut self, js: u32, blitz: NodeId) {
        self.to_blitz.insert(js, blitz);
        self.to_js.insert(blitz, js);
    }

    pub fn blitz(&self, js: u32) -> Option<NodeId> {
        self.to_blitz.get(&js).copied()
    }

    pub fn js(&self, blitz: NodeId) -> Option<u32> {
        self.to_js.get(&blitz).copied()
    }

    fn forget_blitz(&mut self, blitz: NodeId) {
        if let Some(js) = self.to_js.remove(&blitz) {
            self.to_blitz.remove(&js);
            self.measured.remove(&js);
        }
    }

    pub fn measured(&self) -> &HashSet<u32> {
        &self.measured
    }

    pub fn len(&self) -> usize {
        self.to_blitz.len()
    }

    pub fn is_empty(&self) -> bool {
        self.to_blitz.is_empty()
    }
}

/// Counters for one applied batch. Useful for tests and performance tracing.
#[derive(Default, Debug, Clone, Copy)]
pub struct ApplyStats {
    pub ops: usize,
    /// Ops that can change what is on screen. Creating a node, freeing one,
    /// and registering a listener or a measurement cannot.
    pub visible: usize,
    pub created: usize,
    pub dropped: usize,
    /// Nodes taken out of the page with their ancestors. They stay in Blitz
    /// until JavaScript collects their objects (`DROP`).
    pub detached: usize,
}

/// A node and everything under it.
fn subtree_size(doc: &BaseDocument, id: NodeId) -> usize {
    let mut stack = vec![id];
    let mut count = 0;
    while let Some(id) = stack.pop() {
        if let Some(node) = doc.get_node(id) {
            count += 1;
            stack.extend(node.children.iter().copied());
        }
    }
    count
}

fn attr_name(name: &str) -> QualName {
    QualName::new(None, ns!(), LocalName::from(name))
}

fn element_name(name: &str) -> QualName {
    if SVG_ELEMENTS.contains(&name) {
        QualName::new(None, ns!(svg), LocalName::from(name))
    } else {
        QualName::new(None, ns!(html), LocalName::from(name))
    }
}

/// Apply one batch. All mutations go through a single [`blitz_dom::DocumentMutator`],
/// so Blitz sees them as one change and restyles/relayouts once on the next `resolve`.
///
/// A malformed batch (unknown opcode, unknown node) is reported and the offending
/// op skipped; it must never panic, because the batch comes from app code.
pub fn apply_batch(
    doc: &mut BaseDocument,
    nodes: &mut NodeMap,
    listening: &mut HashSet<String>,
    batch: &Batch,
) -> ApplyStats {
    let mut stats = ApplyStats::default();
    let ops = &batch.ops;
    let string = |index: i32| -> &str {
        batch
            .strings
            .get(index as usize)
            .map(String::as_str)
            .unwrap_or("")
    };

    let mut mutator = doc.mutate();
    let mut i = 0;
    // Nodes whose JavaScript objects were collected, freed once the batch has
    // been applied and the mutator flushed. Blitz keeps some work on a node it
    // added for the flush (a form control's owner, a <style>'s sheet, the
    // <title>), and a node can be added and collected in one batch: freed on
    // the spot, the flush would look it up and panic.
    let mut collected = Vec::new();

    // Read `n` operands, or stop if the batch is truncated.
    macro_rules! operands {
        ($n:expr) => {{
            if i + $n >= ops.len() {
                eprintln!("[host] truncated op batch at {i}");
                break;
            }
            let slice = &ops[i + 1..i + 1 + $n];
            i += 1 + $n;
            slice
        }};
    }

    // Resolve a JS id to a live Blitz node, or skip the op.
    macro_rules! node {
        ($js:expr) => {{
            match nodes.blitz($js as u32) {
                Some(id) if mutator.doc.get_node(id).is_some() => id,
                _ => {
                    eprintln!("[host] op refers to unknown node {}", $js);
                    continue;
                }
            }
        }};
    }

    while i < ops.len() {
        let code = ops[i];
        stats.ops += 1;
        if !matches!(
            code,
            CREATE_ELEMENT | CREATE_TEXT | CREATE_COMMENT | DROP | LISTEN | UNLISTEN | MEASURE
        ) {
            stats.visible += 1;
        }

        match code {
            CREATE_ELEMENT => {
                let o = operands!(2);
                let id = mutator.create_element(element_name(string(o[1])), Vec::new());
                nodes.insert(o[0] as u32, id);
                stats.created += 1;
            }
            CREATE_TEXT => {
                let o = operands!(2);
                let id = mutator.create_text_node(string(o[1]));
                nodes.insert(o[0] as u32, id);
                stats.created += 1;
            }
            CREATE_COMMENT => {
                let o = operands!(2);
                let id = mutator.create_comment_node(string(o[1]));
                nodes.insert(o[0] as u32, id);
                stats.created += 1;
            }
            SET_TEXT => {
                let o = operands!(2);
                let id = node!(o[0]);
                mutator.set_node_text(id, string(o[1]));
            }
            SET_ATTR => {
                let o = operands!(3);
                let id = node!(o[0]);
                mutator.set_attribute(id, attr_name(string(o[1])), string(o[2]));
            }
            REMOVE_ATTR => {
                let o = operands!(2);
                let id = node!(o[0]);
                mutator.clear_attribute(id, attr_name(string(o[1])));
            }
            INSERT => {
                let o = operands!(3);
                let parent = node!(o[0]);
                let child = node!(o[1]);
                // A node cannot go inside itself. Blitz does not check, and a
                // cycle in its tree makes the next walk up the ancestors loop forever.
                let mut ancestor = Some(parent);
                let mut cyclic = false;
                while let Some(id) = ancestor {
                    if id == child {
                        cyclic = true;
                        break;
                    }
                    ancestor = mutator.parent_id(id);
                }
                if cyclic {
                    eprintln!(
                        "[host] refused to insert node {} into its own descendant {}",
                        o[1], o[0]
                    );
                    continue;
                }
                // Only use the anchor if it really is a child of `parent`;
                // otherwise fall back to appending rather than corrupting the tree.
                let anchor = (o[2] != 0)
                    .then(|| nodes.blitz(o[2] as u32))
                    .flatten()
                    .filter(|anchor| mutator.parent_id(*anchor) == Some(parent));
                match anchor {
                    Some(anchor) => mutator.insert_nodes_before(anchor, &[child]),
                    None => mutator.append_children(parent, &[child]),
                }
                // A stylesheet the app adds waits for its `load` event (Vite
                // does, for the styles of a chunk it imports). Blitz has
                // asked for the file by now.
                if mutator
                    .element_name(child)
                    .is_some_and(|name| name.local == *"link")
                {
                    nodes.links.push(o[1] as u32);
                }
            }
            REMOVE => {
                let o = operands!(1);
                let id = node!(o[0]);
                stats.detached += subtree_size(mutator.doc, id);
                mutator.remove_node(id);
            }
            DROP => {
                let o = operands!(1);
                // The JS object is gone. Its id goes now: JavaScript never
                // gives an id out twice, but a batch is the app's input, and
                // a later op must not reach the node through it. The node
                // goes after the flush.
                if let Some(id) = nodes.blitz(o[0] as u32) {
                    nodes.forget_blitz(id);
                    collected.push(id);
                }
            }
            LISTEN => {
                let o = operands!(1);
                listening.insert(string(o[0]).to_owned());
            }
            UNLISTEN => {
                let o = operands!(1);
                listening.remove(string(o[0]));
            }
            FOCUS => {
                let o = operands!(1);
                // No focus events from here: the script that asked has
                // already told its own listeners.
                if o[0] == 0 {
                    mutator.doc.clear_focus();
                } else {
                    let id = node!(o[0]);
                    mutator.doc.set_focus_to(id);
                }
            }
            MEASURE => {
                let o = operands!(1);
                node!(o[0]);
                nodes.measured.insert(o[0] as u32);
            }
            SCROLL => {
                let o = operands!(3);
                let id = node!(o[0]);
                let request = scroll::Request::To(id, o[1] as f64, o[2] as f64);
                nodes.scrolls.push(request);
            }
            SCROLL_INTO_VIEW => {
                let o = operands!(3);
                let id = node!(o[0]);
                let (block, inline) = (scroll::Align::from_op(o[1]), scroll::Align::from_op(o[2]));
                nodes
                    .scrolls
                    .push(scroll::Request::IntoView(id, block, inline));
            }
            _ => {
                eprintln!("[host] unknown opcode {code} at {i}; dropping the rest of the batch");
                break;
            }
        }
    }

    // Flushes. A mutator of its own for freeing: the flush of this one would
    // look at its <title> again.
    drop(mutator);
    if !collected.is_empty() {
        let mut mutator = doc.mutate();
        for id in collected {
            // It may already have been freed along with an ancestor.
            if mutator.doc.get_node(id).is_none() {
                continue;
            }
            if mutator.node_has_parent(id) {
                // Still attached: JS collects whole detached trees, so this is a
                // child of a tree whose root will be (or was) dropped separately.
                continue;
            }
            let mut freed = Vec::new();
            mutator.remove_and_drop_node_with(id, &mut |dropped| freed.push(dropped));
            for dropped in freed {
                nodes.forget_blitz(dropped);
                stats.dropped += 1;
            }
        }
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use blitz_dom::DocumentConfig;

    /// Invariant 8: a batch is app input. A bad one is skipped, not obeyed.
    #[test]
    fn a_malformed_batch_is_refused_without_panicking() {
        let mut doc = BaseDocument::new(DocumentConfig::default());
        let mut nodes = NodeMap::default();
        let mut listening = HashSet::new();
        #[rustfmt::skip]
        let ops = vec![
            CREATE_ELEMENT, 2, 0, // <div>
            CREATE_ELEMENT, 3, 1, // <span>
            INSERT, 2, 3, 0,      // span into div
            INSERT, 3, 2, 0,      // div into its own child: a cycle
            INSERT, 2, 2, 0,      // div into itself
            SET_ATTR, 99, 0, 0,   // a node that does not exist
            REMOVE, 99,
            MEASURE, 99,
            MEASURE, 2,
            FOCUS, 99,
            FOCUS, 3,
            SCROLL, 99, 0, 10,
            SCROLL_INTO_VIEW, 99, 0, 3,
            SCROLL, 2, 0, 10,
            77,                   // not an opcode
        ];
        let batch = Batch {
            ops,
            strings: vec!["div".into(), "span".into()],
        };
        apply_batch(&mut doc, &mut nodes, &mut listening, &batch);

        let (div, span) = (nodes.blitz(2).unwrap(), nodes.blitz(3).unwrap());
        assert_eq!(doc.get_node(span).unwrap().parent, Some(div));
        assert_eq!(doc.get_node(div).unwrap().parent, None, "no cycle");
        assert_eq!(nodes.measured().iter().collect::<Vec<_>>(), [&2]);
        assert_eq!(doc.get_focussed_node_id(), Some(span));
        assert_eq!(
            nodes.scrolls.len(),
            1,
            "only the scroll of a node that exists"
        );
    }

    /// A node can be added and collected in one batch: JavaScript makes an
    /// element, puts it on the page, takes it off, and a garbage collection
    /// in the same turn (V8's, or QuickJS's at its allocation threshold)
    /// runs its finalizer before the turn ends. Blitz keeps some work on an
    /// added node for the mutator's flush (a form control's owner, a
    /// <style>'s sheet, the <title>), which must not find it freed.
    #[test]
    fn a_node_added_and_collected_in_one_batch_is_freed_after_the_flush() {
        let mut doc = BaseDocument::new(DocumentConfig::default());
        let mut nodes = NodeMap::default();
        nodes.insert(1, doc.root_node().id);
        let mut listening = HashSet::new();
        #[rustfmt::skip]
        let ops = vec![
            CREATE_ELEMENT, 2, 0, // <input>
            CREATE_ELEMENT, 3, 1, // <style>
            CREATE_ELEMENT, 4, 2, // <title>
            CREATE_TEXT, 5, 3,
            INSERT, 4, 5, 0,
            INSERT, 1, 2, 0,
            INSERT, 1, 3, 0,
            INSERT, 1, 4, 0,
            REMOVE, 2,
            REMOVE, 3,
            REMOVE, 4,
            DROP, 2,
            DROP, 3,
            DROP, 5,              // a child first: it stays until its root goes
            DROP, 4,
        ];
        let batch = Batch {
            ops,
            strings: vec![
                "input".into(),
                "style".into(),
                "title".into(),
                "A title".into(),
            ],
        };
        let stats = apply_batch(&mut doc, &mut nodes, &mut listening, &batch);

        assert_eq!(stats.dropped, 4);
        for js in 2..=5 {
            assert_eq!(nodes.blitz(js), None, "node {js} is still known");
        }
        assert_eq!(doc.root_node().children.len(), 0);
    }

    /// The id goes at the DROP, the node after the flush: an id given out
    /// again in the same batch names the new node, which stays.
    #[test]
    fn an_id_dropped_and_given_out_again_names_the_new_node() {
        let mut doc = BaseDocument::new(DocumentConfig::default());
        let mut nodes = NodeMap::default();
        nodes.insert(1, doc.root_node().id);
        let mut listening = HashSet::new();
        #[rustfmt::skip]
        let ops = vec![
            CREATE_ELEMENT, 2, 0, // <input>
            INSERT, 1, 2, 0,
            REMOVE, 2,
            DROP, 2,
            CREATE_ELEMENT, 2, 1, // <div>, under the same id
            INSERT, 1, 2, 0,
        ];
        let batch = Batch {
            ops,
            strings: vec!["input".into(), "div".into()],
        };
        let stats = apply_batch(&mut doc, &mut nodes, &mut listening, &batch);

        assert_eq!(stats.dropped, 1);
        let div = nodes.blitz(2).expect("the id names the <div>");
        let node = doc.get_node(div).expect("the <div> was not freed");
        assert_eq!(&*node.element_data().unwrap().name.local, "div");
        assert_eq!(doc.root_node().children, vec![div]);
    }

    /// Blitz builds no boxes below a `display: none` box and never looks at
    /// the layout children it gave them before. A child changed and freed
    /// down there (the Tracker's list column behind the detail pane, once
    /// V8 collects the old labels) left an id that is no longer a node, and
    /// Blitz's debug check of the layout tree panicked on it.
    #[test]
    fn a_node_freed_below_a_hidden_box_is_not_a_layout_child() {
        use blitz_traits::shell::{ColorScheme, Viewport};

        fn run(doc: &mut BaseDocument, nodes: &mut NodeMap, ops: Vec<i32>) {
            let strings = [
                "html",
                "div",
                "p",
                "section",
                "style",
                "display: none",
                "aside",
            ];
            let batch = Batch {
                ops,
                strings: strings.map(String::from).to_vec(),
            };
            apply_batch(doc, nodes, &mut HashSet::new(), &batch);
            doc.resolve(0.0);
        }
        let children = |doc: &BaseDocument, nodes: &NodeMap| {
            let p = doc.get_node(nodes.blitz(4).unwrap()).unwrap();
            p.layout_children
                .borrow()
                .clone()
                .unwrap_or_default()
                .to_vec()
        };

        let mut doc = BaseDocument::new(DocumentConfig::default());
        doc.set_viewport(Viewport::new(200, 100, 1.0, ColorScheme::Light));
        let mut nodes = NodeMap::default();
        nodes.insert(1, doc.root_node().id);
        #[rustfmt::skip]
        run(&mut doc, &mut nodes, vec![
            CREATE_ELEMENT, 2, 0, // <html>
            CREATE_ELEMENT, 3, 1, // <div>
            CREATE_ELEMENT, 4, 2, // <p>, laid out while it shows
            CREATE_ELEMENT, 5, 3, // <section>
            INSERT, 1, 2, 0,
            INSERT, 2, 3, 0,
            INSERT, 3, 4, 0,
            INSERT, 4, 5, 0,
        ]);
        assert_eq!(children(&doc, &nodes), [nodes.blitz(5).unwrap()]);

        run(&mut doc, &mut nodes, vec![SET_ATTR, 3, 4, 5]);
        #[rustfmt::skip]
        run(&mut doc, &mut nodes, vec![
            REMOVE, 5,
            DROP, 5,
            CREATE_ELEMENT, 6, 6, // <aside>
            INSERT, 4, 6, 0,
        ]);
        run(&mut doc, &mut nodes, vec![REMOVE_ATTR, 3, 4]);
        assert_eq!(
            children(&doc, &nodes),
            [nodes.blitz(6).unwrap()],
            "shown again, it is laid out anew"
        );
    }
}
