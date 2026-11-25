//! Helpers for driving a [`SvelteDocument`] without a window: query the tree,
//! synthesise input, read text. These go through the same code paths as real
//! input (hit testing, Blitz's event driver, the JS dispatcher).

use blitz_dom::{Document, NodeId};
use blitz_traits::events::{
    BlitzKeyEvent, BlitzPointerEvent, BlitzPointerId, BlitzWheelDelta, BlitzWheelEvent, KeyState,
    MouseEventButton, MouseEventButtons, PointerCoords, PointerDetails, UiEvent,
};
use blitz_traits::shell::{ColorScheme, Viewport};
use keyboard_types::{Code, Key, Location, Modifiers, NamedKey};

use crate::SvelteDocument;

impl SvelteDocument {
    /// Set the window size in CSS pixels and lay the document out.
    pub fn layout(&mut self, width: u32, height: u32) {
        self.base_mut()
            .set_viewport(Viewport::new(width, height, 1.0, ColorScheme::Light));
        self.resolve();
    }

    /// Restyle and relayout after a change.
    pub fn resolve(&mut self) {
        self.resolve_layout();
        // As a window does after each frame.
        self.after_layout();
    }

    /// Let time pass: timers that come due fire, `requestAnimationFrame`
    /// callbacks run, and CSS transitions and animations move on by `seconds`.
    ///
    /// In steps of 20 ms, each a turn and a frame, as a window would: one
    /// timer often starts the next.
    pub fn wait(&mut self, seconds: f64) {
        const STEP: f64 = 0.02;
        let mut left = seconds;
        while left > 0.0 {
            let step = left.min(STEP);
            self.advance_time(step);
            self.settle();
            // A frame per step, as a window draws them while something waits for one.
            self.begin_frame();
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
        let base = self.base();
        let node = base.get_node(node)?;
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
        self.click_at_with(x, y, Modifiers::empty());
    }

    /// [`click_at`](Self::click_at) with modifier keys held (Shift+click).
    pub fn click_at_with(&mut self, x: f32, y: f32, mods: Modifiers) {
        let at = |buttons| pointer(x, y, buttons, mods);
        self.handle_ui_event(UiEvent::PointerMove(at(MouseEventButtons::None)));
        self.handle_ui_event(UiEvent::PointerDown(at(MouseEventButtons::Primary)));
        self.handle_ui_event(UiEvent::PointerUp(at(MouseEventButtons::None)));
        self.resolve();
    }

    /// Move the pointer to a point, with no button held.
    pub fn move_to(&mut self, x: f32, y: f32) {
        self.handle_ui_event(UiEvent::PointerMove(pointer(
            x,
            y,
            MouseEventButtons::None,
            Modifiers::empty(),
        )));
        self.resolve();
    }

    /// Turn the wheel with the pointer at a point, by `dx`, `dy` CSS pixels.
    /// Positive `dy` scrolls the content down, as `scrollBy` does. Blitz
    /// scrolls the box under the pointer, or the nearest one around it that
    /// can still move.
    pub fn wheel(&mut self, x: f32, y: f32, dx: f64, dy: f64) {
        self.move_to(x, y);
        let at = pointer(x, y, MouseEventButtons::None, Modifiers::empty());
        self.handle_ui_event(UiEvent::Wheel(BlitzWheelEvent {
            // A wheel event says which way the content moves.
            delta: BlitzWheelDelta::Pixels(-dx, -dy),
            coords: at.coords,
            buttons: at.buttons,
            mods: at.mods,
            element: Default::default(),
        }));
        self.resolve();
    }

    /// Press and release a key, as a keyboard would. `key` is what
    /// `KeyboardEvent.key` says: a name ("Escape", "ArrowDown", "Tab",
    /// "Enter") or the character typed ("a", " ").
    pub fn press(&mut self, key: &str) {
        self.press_with(key, Modifiers::empty());
    }

    /// [`press`](Self::press) with modifier keys held.
    pub fn press_with(&mut self, key: &str, modifiers: Modifiers) {
        for state in [KeyState::Pressed, KeyState::Released] {
            let event = key_event(key, modifiers, state);
            self.handle_ui_event(match state {
                KeyState::Pressed => UiEvent::KeyDown(event),
                KeyState::Released => UiEvent::KeyUp(event),
            });
            // On macOS the system turns editing keys into commands, and Blitz
            // leaves Backspace to that.
            if cfg!(target_os = "macos") && state == KeyState::Pressed && key == "Backspace" {
                self.handle_ui_event(UiEvent::AppleStandardKeybinding("deleteBackward:".into()));
            }
        }
        self.resolve();
    }

    /// Type text, one key per character.
    pub fn type_text(&mut self, text: &str) {
        for character in text.chars() {
            self.press(&character.to_string());
        }
    }

    /// Click the centre of a node. The document must have been laid out.
    pub fn click(&mut self, node: NodeId) {
        self.click_with(node, Modifiers::empty());
    }

    /// [`click`](Self::click) with modifier keys held.
    pub fn click_with(&mut self, node: NodeId, mods: Modifiers) {
        let (x, y, width, height) = self.rect(node).expect("node has no layout");
        self.click_at_with((x + width / 2.0) as f32, (y + height / 2.0) as f32, mods);
    }
}

fn key_event(key: &str, modifiers: Modifiers, state: KeyState) -> BlitzKeyEvent {
    // A name that `Key` does not know is a character.
    let key: Key = key.parse().unwrap_or_else(|_| Key::Character(key.into()));
    let (code, text) = match &key {
        Key::Character(typed) => {
            let character = typed.chars().next().unwrap_or(' ');
            let code = match character {
                'a'..='z' | 'A'..='Z' => format!("Key{}", character.to_ascii_uppercase()),
                '0'..='9' => format!("Digit{character}"),
                ' ' => "Space".to_owned(),
                _ => String::new(),
            };
            (code, Some(typed.as_str().into()))
        }
        Key::Named(NamedKey::Enter) => ("Enter".to_owned(), Some("\r".into())),
        Key::Named(named) => (named.to_string(), None),
    };
    BlitzKeyEvent {
        key,
        code: code.parse().unwrap_or(Code::Unidentified),
        modifiers,
        location: Location::Standard,
        is_auto_repeating: false,
        is_composing: false,
        state,
        text,
    }
}

fn pointer(x: f32, y: f32, buttons: MouseEventButtons, mods: Modifiers) -> BlitzPointerEvent {
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
        mods,
        details: PointerDetails::default(),
        element: Default::default(),
        active_pointers: Default::default(),
    }
}
