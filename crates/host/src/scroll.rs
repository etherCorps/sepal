//! Scroll positions: what the app asks for (`SCROLL`, `SCROLL_INTO_VIEW` in
//! `ops.rs`), and what it is told when one changes.
//!
//! Blitz scrolls on its own when the wheel or a scrollbar moves a box. The
//! host notes which boxes moved, and at the end of the turn tells the app
//! with a `scroll` event on each (see `SvelteDocument::report_scrolls`).

use blitz_dom::{BaseDocument, NodeId, ScrollBehavior};

/// A scroll the app asked for. Applied once the page is laid out, because the
/// range it is clamped to depends on what the same batch put in the box.
#[derive(Debug, Clone, Copy)]
pub enum Request {
    /// To this offset, `(left, top)`.
    To(NodeId, f64, f64),
    /// Until the node is in view in every box that scrolls it: how to align
    /// it vertically and horizontally, as [`Align`].
    IntoView(NodeId, Align, Align),
}

/// `block` and `inline` of `scrollIntoView()`. The numbers are the protocol's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Align {
    Start = 0,
    Center = 1,
    End = 2,
    Nearest = 3,
}

impl Align {
    pub fn from_op(value: i32) -> Self {
        match value {
            1 => Align::Center,
            2 => Align::End,
            3 => Align::Nearest,
            _ => Align::Start,
        }
    }

    /// The scroll offset that puts `[start, start + size)` where this says,
    /// in a scrollport `port` long that is now at `current`.
    fn offset(self, current: f64, port: f64, start: f64, size: f64) -> f64 {
        let end = start + size;
        match self {
            Align::Start => start,
            Align::Center => start - (port - size) / 2.0,
            Align::End => end - port,
            Align::Nearest => {
                let visible = start >= current && end <= current + port;
                let covers = start <= current && end >= current + port;
                if visible || covers {
                    current
                } else if (start - current).abs() < (end - port - current).abs() {
                    start
                } else {
                    end - port
                }
            }
        }
    }
}

/// `[scrollLeft, scrollTop, scrollWidth, scrollHeight, clientWidth,
/// clientHeight]` of a node, as the DOM defines them.
pub fn metrics(doc: &BaseDocument, id: NodeId) -> Option<[f64; 6]> {
    let node = doc.get_node(id)?;
    let offset = node.scroll_offset();
    let layout = node.final_layout();
    let client_width = (layout.size.width
        - layout.border.left
        - layout.border.right
        - layout.scrollbar_size.width)
        .max(0.0) as f64;
    let client_height = (layout.size.height
        - layout.border.top
        - layout.border.bottom
        - layout.scrollbar_size.height)
        .max(0.0) as f64;
    // Taffy's "scroll width" is how far the box can scroll, the DOM's is the
    // length of what it scrolls over.
    Some([
        offset.x,
        offset.y,
        client_width + layout.scroll_width() as f64,
        client_height + layout.scroll_height() as f64,
        client_width,
        client_height,
    ])
}

/// Where a node's border box is on screen, `(x, y, width, height)`.
///
/// Blitz's `get_client_bounding_rect` moves a box by its own scroll offset,
/// as if it were inside itself; in a browser scrolling a box moves what is
/// in it, not the box. That is put right here.
pub fn client_rect(doc: &BaseDocument, id: NodeId) -> Option<(f64, f64, f64, f64)> {
    let rect = doc.get_client_bounding_rect(id)?;
    let own = doc.get_node(id)?.scroll_offset();
    Some((rect.x + own.x, rect.y + own.y, rect.width, rect.height))
}

/// Apply a scroll the app asked for. Every box whose offset changed is added
/// to `scrolled`, so that the app is told.
pub fn apply(doc: &mut BaseDocument, request: Request, scrolled: &mut Vec<NodeId>) {
    match request {
        Request::To(id, left, top) => scroll_to(doc, id, left, top, scrolled),
        Request::IntoView(id, block, inline) => {
            // Innermost box first: once it has moved, the boxes around it
            // see the node where it now is.
            let mut ancestor = doc.get_node(id).and_then(|node| node.parent);
            while let Some(container) = ancestor {
                ancestor = doc.get_node(container).and_then(|node| node.parent);
                let Some(node) = doc.get_node(container) else {
                    break;
                };
                let layout = node.final_layout();
                // Nothing in it reaches past its edges: there is nothing to scroll.
                if layout.scroll_width() <= 0.0 && layout.scroll_height() <= 0.0 {
                    continue;
                }
                let (border_left, border_top) =
                    (layout.border.left as f64, layout.border.top as f64);
                let (Some(target), Some(port), Some(now)) = (
                    client_rect(doc, id),
                    client_rect(doc, container),
                    metrics(doc, container),
                ) else {
                    continue;
                };
                // Where the node is in what the box scrolls over.
                let x = target.0 - port.0 - border_left + now[0];
                let y = target.1 - port.1 - border_top + now[1];
                let left = inline.offset(now[0], now[4], x, target.2);
                let top = block.offset(now[1], now[5], y, target.3);
                scroll_to(doc, container, left, top, scrolled);
            }
        }
    }
}

fn scroll_to(doc: &mut BaseDocument, id: NodeId, left: f64, top: f64, scrolled: &mut Vec<NodeId>) {
    let Some(before) = doc.get_node(id).map(|node| *node.scroll_offset()) else {
        return;
    };
    // Clamped by Blitz, to what the box can scroll. A box with `overflow:
    // visible` cannot, and stays where it is.
    doc.scroll_to(id, left, top, ScrollBehavior::Instant);
    let after = doc.get_node(id).map(|node| *node.scroll_offset());
    if after != Some(before) && !scrolled.contains(&id) {
        scrolled.push(id);
    }
}

#[cfg(test)]
mod tests {
    use super::Align;

    #[test]
    fn alignment() {
        // A 100px port at 0, a 20px row at 200.
        assert_eq!(Align::Start.offset(0.0, 100.0, 200.0, 20.0), 200.0);
        assert_eq!(Align::End.offset(0.0, 100.0, 200.0, 20.0), 120.0);
        assert_eq!(Align::Center.offset(0.0, 100.0, 200.0, 20.0), 160.0);
        assert_eq!(Align::Nearest.offset(0.0, 100.0, 200.0, 20.0), 120.0);
        assert_eq!(Align::Nearest.offset(300.0, 100.0, 200.0, 20.0), 200.0);
        // Already in view: stays.
        assert_eq!(Align::Nearest.offset(150.0, 100.0, 200.0, 20.0), 150.0);
    }
}
