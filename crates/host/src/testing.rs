//! Helpers for driving a [`SvelteDocument`] without a window: query the tree,
//! synthesise input, read text. These go through the same code paths as real
//! input (hit testing, Blitz's event driver, the JS dispatcher).

use blitz_dom::{Document, NodeId};
use blitz_traits::events::{
    BlitzPointerEvent, BlitzPointerId, MouseEventButton, MouseEventButtons, PointerCoords,
    PointerDetails, UiEvent,
};
use blitz_traits::shell::{ColorScheme, Viewport};

use crate::SvelteDocument;

impl SvelteDocument {
    /// Set the window size in CSS pixels and lay the document out.
    pub fn layout(&mut self, width: u32, height: u32) {
        let base = self.base_mut();
        base.set_viewport(Viewport::new(width, height, 1.0, ColorScheme::Light));
        self.resolve();
    }

    /// Restyle and relayout after a change.
    pub fn resolve(&mut self) {
        let time = self.headless_time;
        self.base_mut().resolve(time);
        // As a window does after each frame.
        self.after_layout();
    }

    /// Let time pass: timers that come due fire, and CSS transitions and
    /// animations move on by `seconds`.
    ///
    /// In steps, as a window would: one timer often starts the next.
    pub fn wait(&mut self, seconds: f64) {
        const STEP: f64 = 0.02;
        let mut left = seconds;
        while left > 0.0 {
            let step = left.min(STEP);
            std::thread::sleep(std::time::Duration::from_secs_f64(step));
            self.headless_time += step;
            self.settle();
            self.resolve();
            left -= step;
        }
    }

    pub fn query(&self, selector: &str) -> Option<NodeId> {
        self.base().query_selector(selector).ok().flatten()
    }

    pub fn query_all(&self, selector: &str) -> Vec<NodeId> {
        self.base()
            .query_selector_all(selector)
            .map(|nodes| nodes.to_vec())
            .unwrap_or_default()
    }

    /// The text of a node and its descendants, with whitespace collapsed.
    pub fn text(&self, node: NodeId) -> String {
        let raw = self
            .base()
            .get_node(node)
            .map(|node| node.text_content())
            .unwrap_or_default();
        raw.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    pub fn attr(&self, node: NodeId, name: &str) -> Option<String> {
        let node = self.base().get_node(node)?;
        let element = node.element_data()?;
        element
            .attrs()
            .iter()
            .find(|attr| &*attr.name.local == name)
            .map(|attr| attr.value.to_string())
    }

    /// Border-box rectangle of a node in viewport coordinates: `(x, y, width, height)`.
    pub fn rect(&self, node: NodeId) -> Option<(f64, f64, f64, f64)> {
        let rect = self.base().get_client_bounding_rect(node)?;
        Some((rect.x, rect.y, rect.width, rect.height))
    }

    /// Press and release the primary mouse button at a point. Blitz hit-tests the
    /// point and synthesises `pointerdown`/`mousedown`/`pointerup`/`mouseup`/`click`.
    pub fn click_at(&mut self, x: f32, y: f32) {
        self.handle_ui_event(UiEvent::PointerMove(pointer(x, y, MouseEventButtons::None)));
        self.handle_ui_event(UiEvent::PointerDown(pointer(
            x,
            y,
            MouseEventButtons::Primary,
        )));
        self.handle_ui_event(UiEvent::PointerUp(pointer(x, y, MouseEventButtons::None)));
        self.resolve();
    }

    /// Move the pointer to a point, with no button held.
    pub fn move_to(&mut self, x: f32, y: f32) {
        self.handle_ui_event(UiEvent::PointerMove(pointer(x, y, MouseEventButtons::None)));
        self.resolve();
    }

    /// Click the centre of a node. The document must have been laid out.
    pub fn click(&mut self, node: NodeId) {
        let (x, y, width, height) = self.rect(node).expect("node has no layout");
        self.click_at((x + width / 2.0) as f32, (y + height / 2.0) as f32);
    }
}

fn pointer(x: f32, y: f32, buttons: MouseEventButtons) -> BlitzPointerEvent {
    BlitzPointerEvent {
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
        buttons,
        mods: Default::default(),
        details: PointerDetails::default(),
        element: Default::default(),
        active_pointers: Default::default(),
    }
}
