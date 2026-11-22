//! A [`vello_gpu`] backend for the [`anyrender`] 2D drawing abstraction
#![cfg_attr(docsrs, feature(doc_cfg))]

mod filters;
// svelte-blitz patch
#[cfg(not(target_arch = "wasm32"))]
mod offscreen;
mod scene;
#[cfg(all(target_arch = "wasm32", feature = "webgl"))]
mod webgl_scene;
mod window_renderer;

pub use scene::ImageManager;
pub use scene::VelloHybridScenePainter;
#[cfg(all(target_arch = "wasm32", feature = "webgl"))]
pub use webgl_scene::*;
pub use window_renderer::*;

pub use wgpu;

// svelte-blitz patch: see `scene.rs`, `draw_glyphs`.
static GLYPH_CACHE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// Whether glyphs are rasterised once into an atlas and reused (the default
/// here), or again for every frame (what upstream does).
pub fn glyph_cache() -> bool {
    GLYPH_CACHE.load(std::sync::atomic::Ordering::Relaxed)
}

/// Turn the glyph atlas on or off, for everything drawn from now on.
pub fn set_glyph_cache(enabled: bool) {
    GLYPH_CACHE.store(enabled, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(not(target_arch = "wasm32"))]
pub use offscreen::render_to_rgba;
