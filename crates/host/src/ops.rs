//! Applies op batches from the JS renderer to a Blitz document.
//!
//! Keep the opcodes in sync with `packages/core/src/protocol.js`.

use std::collections::{HashMap, HashSet};

use blitz_dom::{BaseDocument, LocalName, NodeId, QualName, ns};

use crate::js::Batch;

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

/// The JS id of the mount target, mapped to `<body>`.
pub const ROOT_ID: u32 = 1;

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
        }
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
    pub created: usize,
    pub dropped: usize,
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
            }
            REMOVE => {
                let o = operands!(1);
                let id = node!(o[0]);
                mutator.remove_node(id);
            }
            DROP => {
                let o = operands!(1);
                // The JS object is gone, so the id can never be referenced again.
                // It may already have been freed along with an ancestor.
                let Some(id) = nodes.blitz(o[0] as u32) else {
                    continue;
                };
                if mutator.doc.get_node(id).is_none() {
                    nodes.forget_blitz(id);
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
            LISTEN => {
                let o = operands!(1);
                listening.insert(string(o[0]).to_owned());
            }
            UNLISTEN => {
                let o = operands!(1);
                listening.remove(string(o[0]));
            }
            _ => {
                eprintln!("[host] unknown opcode {code} at {i}; dropping the rest of the batch");
                break;
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
    }
}
