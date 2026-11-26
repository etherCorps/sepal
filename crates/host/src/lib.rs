//! Runs a Svelte app compiled for the `sepal` renderer on top of Blitz.
//!
//! ```text
//! index.html  --parsed-->  blitz-dom
//! its scripts (Svelte + renderer)  --op batches-->  SvelteDocument  -->  blitz-dom  -->  paint
//!                                  <---events----
//! ```

pub mod api;
pub mod app;
pub mod bundle;
pub mod bytecode;
pub mod document;
mod hud;
pub mod js;
pub mod net;
pub mod ops;
mod page;
pub mod scroll;
pub mod testing;
mod timers;
pub mod updater;

#[cfg(feature = "screenshot")]
pub mod screenshot;

#[cfg(feature = "window")]
pub mod window;

pub use document::{SvelteDocument, TurnStats};
