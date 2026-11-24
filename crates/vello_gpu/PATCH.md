# Patched copy of vello_gpu 0.3.0

This is the crates.io source of `vello_gpu` 0.3.0 (Apache-2.0 OR MIT,
https://github.com/linebender/vello), the GPU renderer under
`anyrender_vello_hybrid`, with two changes. `crates/host/Cargo.toml` swaps it
in through `[patch.crates-io]`. Both are marked `sepal patch`.

## The changes

**1. Layer textures shrink again.** `src/render/wgpu/mod.rs`, `render_scene`
and `prepare_intermediate_textures`: vello_gpu draws layers (opacity, clips,
blend modes) into intermediate textures that upstream only ever grows, in size
and in number. All of them share one size, that of the largest layer drawn so
far, so a single layer as large as the window (a Dialog's dimmed backdrop)
made every one of them that large for as long as the window lived: on the
Tracker, opening the New issue dialog once left 42 MB behind.

Now a frame that needs less than a quarter of the current area recreates them
at the size it needs, and pages a frame does not use are dropped (with the
bind groups that would keep them alive). The quarter keeps a layer that shrinks
a little each frame, as in an animation, from reallocating them every frame.
Measured on the Tracker in a window (1200x800 at 2x, `footprint`): with the
dialog open 434 MB, closed 390 MB (was 422 MB), and the same after opening it
again. The window's own frames, captured before and after, are identical.

**2. A root offset.** `src/render/wgpu/mod.rs`, `Renderer::set_root_offset`,
`Programs::root_offset` and `maybe_update_config_buffer`: the root view's config
already has `strip_offset_x`/`strip_offset_y` (the shader adds them to every
strip's position before mapping to the target), always 0 upstream. The caller
can now set them, so a scene the size of a region is drawn into that region of a
window-sized target (`render_size` stays the target's) without clearing the rest
(`TargetInit::SrcOver`). Used by anyrender_vello_hybrid's damage tracking.
It applies to renders into the user's surface only (`Programs::target_offset`,
set in `render_scene`): glyphs are rasterised into atlas pages through the same
root config (`RootTarget::AtlasLayer`), and with the offset applied there, every
glyph first drawn in a partial frame landed outside its slot and stayed blank.

## Updating

Do not edit anything else here, and do not run `cargo fmt` on it. To move to a
new version, replace the directory with that version's source and reapply the
changes (search for `sepal patch`), unless upstream shrinks the
textures itself by then.
