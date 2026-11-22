//! Runs a Svelte app compiled for the `@ethercorps/svelte-blitz` renderer on top of Blitz.
//!
//! ```text
//! app.js (Svelte + renderer)  --op batches-->  SvelteDocument  -->  blitz-dom  -->  paint
//!                             <---events----
//! ```

pub mod app;
pub mod bundle;
pub mod document;
pub mod js;
pub mod net;
pub mod ops;
pub mod testing;
mod timers;

#[cfg(feature = "screenshot")]
pub mod screenshot;

#[cfg(feature = "window")]
pub mod window;

pub use document::{SvelteDocument, TurnStats};
