//! The page: `index.html`, parsed into the document before any script runs.
//!
//! The host takes what Vite and SvelteKit write: a head with a title,
//! `<meta>`, stylesheets (`<link>` and `<style>`) and `modulepreload`, markup
//! in the body, inline classic scripts and module scripts. Blitz applies the
//! stylesheets as it parses. What else a browser would do with a page
//! (`on*` attributes, iframes, classic scripts from a file, import maps) is
//! left alone here, and a dev build says so once.
//!
//! Every node parsed is numbered for the JS mirror, `1..` in document order,
//! and handed to JavaScript once, as `__sb_page`, before the first script runs
//! (`page.js`). These are the only node ids the host hands out; JavaScript
//! allocates the rest, from where these end.

use std::collections::HashSet;

use blitz_dom::{BaseDocument, NodeData, NodeId, local_name};
use blitz_traits::net::Url;
use html5ever::tendril::TendrilSink;

use crate::net::ORIGIN;
use crate::ops::{CREATE_COMMENT, CREATE_ELEMENT, CREATE_TEXT, INSERT, SET_ATTR};

/// A script of the page, in the order it runs.
pub enum Script {
    /// An inline classic script: its `<script>` element, and its code.
    Classic { node: u32, code: String },
    /// A module script: its URL, and its code when it is inline.
    Module { url: String, code: Option<String> },
}

pub struct Page {
    /// Every node parsed: its JS id, and the Blitz node.
    pub nodes: Vec<(u32, NodeId)>,
    /// The JS expression that sets `__sb_page` up (`page.js`): the parsed
    /// tree as ops in the renderer's own vocabulary, each node's parent, and
    /// the first id JavaScript may allocate.
    pub seed: String,
    /// Classic scripts first, then module scripts, each in document order:
    /// the order a browser runs them in, modules being deferred.
    pub scripts: Vec<Script>,
}

/// What is set up before any script of the page runs: `__sb_page`, and a
/// stand-in `__sb` until the app's runtime replaces it.
pub const PREAMBLE: &str = include_str!("page.js");

/// Parse `html` into `doc`, which has nothing in it yet. In `dev`, what the
/// host leaves out of the page is reported.
pub fn parse(doc: &mut BaseDocument, html: &str, dev: bool) -> Page {
    {
        let mut mutator = doc.mutate();
        let sink = blitz_html::DocumentHtmlParser::new(&mut mutator);
        let options = html5ever::ParseOpts {
            tree_builder: html5ever::tree_builder::TreeBuilderOpts {
                // Scripts run, so <noscript> is text, as in a browser.
                scripting_enabled: true,
                drop_doctype: true,
                ..Default::default()
            },
            ..Default::default()
        };
        html5ever::parse_document(sink, options).one(html);
    }

    let mut warned = HashSet::new();
    let mut ignore = |what: &'static str| {
        if dev && warned.insert(what) {
            eprintln!("[host] index.html: {what} are not supported here and were ignored");
        }
    };
    let page = Url::parse(ORIGIN).expect("the app's origin is a URL");

    let (mut ops, mut strings) = (Vec::new(), Vec::<String>::new());
    let mut string = |text: &str| {
        strings.push(text.to_owned());
        strings.len() as i32 - 1
    };
    let mut parents = vec![0];
    let mut nodes = Vec::new();
    let (mut classic, mut modules) = (Vec::new(), Vec::new());

    // Depth first, in document order: (node, its parent's JS id).
    let mut stack: Vec<(NodeId, u32)> = doc
        .root_node()
        .children
        .iter()
        .rev()
        .map(|&id| (id, 0))
        .collect();
    while let Some((id, parent)) = stack.pop() {
        let Some(node) = doc.get_node(id) else {
            continue;
        };
        let js = parents.len() as u32;
        match &node.data {
            NodeData::Element(element) => {
                ops.extend([CREATE_ELEMENT, js as i32, string(&element.name.local)]);
                for attr in element.attrs() {
                    if attr.name.local.starts_with("on") {
                        ignore("event handler attributes (onclick and the like)");
                    }
                    ops.extend([
                        SET_ATTR,
                        js as i32,
                        string(&attr.name.local),
                        string(&attr.value),
                    ]);
                }
                match &*element.name.local {
                    "script" => {
                        let kind = element
                            .attr(local_name!("type"))
                            .map(|kind| kind.trim().to_ascii_lowercase())
                            .unwrap_or_default();
                        let src = element.attr(local_name!("src"));
                        let javascript = matches!(
                            kind.as_str(),
                            "" | "text/javascript"
                                | "application/javascript"
                                | "text/ecmascript"
                                | "application/ecmascript"
                        );
                        if element.attr(local_name!("nomodule")).is_some() {
                            // For browsers without modules.
                        } else if kind == "module" {
                            let code = src.is_none().then(|| node.text_content());
                            let url = match src {
                                Some(src) => page.join(src).ok().filter(crate::net::in_app),
                                // Its own name; `import.meta.url` is the page's.
                                None => Url::parse(&format!("{ORIGIN}#inline-{js}")).ok(),
                            };
                            match url {
                                Some(url) => modules.push(Script::Module {
                                    url: url.into(),
                                    code,
                                }),
                                None => eprintln!(
                                    "[host] index.html: refused the module {}: scripts load only from the app folder",
                                    src.unwrap_or_default()
                                ),
                            }
                        } else if javascript && src.is_some() {
                            ignore(
                                "classic scripts from a file (<script src> without type=\"module\")",
                            );
                        } else if javascript {
                            classic.push(Script::Classic {
                                node: js,
                                code: node.text_content(),
                            });
                        } else if kind == "importmap" {
                            ignore("import maps");
                        }
                        // Anything else is data for scripts to read.
                    }
                    "iframe" => ignore("iframes"),
                    _ => {}
                }
            }
            NodeData::Text(text) => ops.extend([CREATE_TEXT, js as i32, string(&text.content)]),
            NodeData::Comment { contents } => {
                ops.extend([CREATE_COMMENT, js as i32, string(&contents[..])])
            }
            // Nothing else comes out of the parser.
            _ => continue,
        }
        if parent != 0 {
            ops.extend([INSERT, parent as i32, js as i32, 0]);
        }
        parents.push(parent);
        nodes.push((js, id));
        stack.extend(node.children.iter().rev().map(|&child| (child, js)));
    }

    let seed = serde_json::json!({
        "ops": ops,
        "strings": strings,
        "parents": parents,
        "next_id": parents.len(),
    })
    .to_string();
    classic.append(&mut modules);
    Page {
        nodes,
        seed,
        scripts: classic,
    }
}
