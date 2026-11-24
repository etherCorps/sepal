# Patched copy of wgpu-hal 30.0.2

This is the crates.io source of `wgpu-hal` 30.0.2 (MIT OR Apache-2.0,
https://github.com/gfx-rs/wgpu), with three changes, in its Metal backend only.
`crates/host/Cargo.toml` swaps it in through `[patch.crates-io]`. All of it is
marked `svelte-blitz patch`.

## The changes

**1. Clears and uploads in render passes, not blits.** `src/metal/render_copy.rs`
(new), and hooks in `src/metal/command.rs` (`clear_buffer`,
`copy_buffer_to_buffer`, `copy_buffer_to_texture`, `render_copy_encoder`),
`src/metal/device.rs` (`create_texture`) and `src/metal/mod.rs`
(`AdapterShared`).

On Apple GPUs the driver commits about 120 MB the first time a process runs a
render pass, and about 80 MB more the first time it runs any blit or compute
work, for as long as frames are drawn (it is reclaimed 1-2 s after the last
one). Measured with plain Metal programs, outside wgpu: a 16x16 clear costs the
120 MB whatever the target size, and adding one 512 KB `fillBuffer` or one tiny
compute dispatch to it costs the 80 MB. wgpu does blit work all the time:
every `Queue::write_buffer` and `write_texture` is a copy from a staging buffer,
and resources are zero-initialised with `fillBuffer` and copies. A window that
never needs a blit paid for one.

The same work done in render encoders does not trigger it (also measured):
buffer words are written by a vertex function with rasterization off
(`copy_words`, `zero_words`, 32 words per vertex: one per vertex cost
measurably more, 1 to 128 measured the same), and texels by a fragment function that reads the
source buffer, drawn over the region (`to_rgba8unorm`, `to_rgba32uint`, the two
formats vello_gpu uploads to). It stays on the GPU timeline, in order with
everything else in the command buffer; doing the copies on the CPU instead
broke text, because a texture's zero-initialisation then landed after the
glyphs written into it. Textures of those formats that are copied into get the
render-target usage so they can be drawn into. Other formats, 3D textures,
texture-to-texture and texture-to-buffer copies still use blits. Apple7 GPUs
(M1) and later only; `SVELTE_BLITZ_GPU_COPIES=blit` turns it off.

Measured on the Tracker (release, bench mode, a clock repainting once a
second, `footprint`): 377 MB with blits, 285 MB without. Window captures of the
Issues list, the New issue dialog, the Board and Insights, and `gpushot:`
renders of an image drawn from the atlas and of one in a texture of its own,
are byte-identical either way.

**2. A log of what each frame asks of the GPU.** `src/metal/gpu_log.rs` (new),
and one call each in `src/metal/command.rs` (`begin_encoding`, `clear_buffer`,
the four copies, `begin_render_pass`, `begin_compute_pass`) and
`src/metal/mod.rs` (`Queue::submit`, `Queue::present`). With
`SVELTE_BLITZ_GPU_LOG=1` every command buffer, render or compute pass (with
its targets' sizes, formats and load/store actions), clear and copy (with its
size, and whether change 1 draws it in a render pass) is printed on stderr as
`[gpu] …`, and so is every submit and present. A frame is what lies between two
presents. Off (the default), it costs one cached check per call. For
measuring, not for shipping builds to read.

**3. Empty command buffers are not committed.** `src/metal/command.rs`
(`CommandEncoder::encoded`, set wherever an encoder is opened on the command
buffer, and `end_encoding`), `src/metal/device.rs` (`create_command_encoder`)
and `src/metal/mod.rs` (`CommandBuffer::empty`, `Queue::submit`). wgpu-core
encodes a "Pre Pass" command buffer before every pass, a "Transit" one per
submit and a "PendingWrites" one when there are queued writes; on Metal,
where transitions do nothing, most of these hold nothing, and every committed
command buffer costs GPU time (about 0.15 ms each at the clocks an idle app
runs at). A command buffer nothing was encoded into is now dropped instead of
committed; the fence's signal goes on the last one that is committed, or on a
command buffer of its own when none is, as for an empty submit upstream. It is
counted out of `command_buffer_created_not_submitted` when it is dropped (the
existing `Drop for CommandBuffer`; counting it at submit too wrapped the
counter, and the next `begin_encoding` reported the device lost). A command
buffer handed out through `raw_command_buffer()` counts as used, and with
strict event sync every one does (its relay wait is work).

A clock tick on the Tracker commits 6 command buffers instead of 10. Measured
against the code without it in one interleaved session (compare harness,
1,000 issues, 3 runs each): GPU time the same within noise (clock 14.1 -> 13.4
ms, New issue dialog 67.1 -> 65.8, view switches 62.6 -> 61.9, scrolling 518 ->
546 with runs spread 364-555), CPU time 268 -> 250 ms for the dialog and
unchanged elsewhere. Empty command buffers turned out cheap; it is kept for
the CPU it saves and because change 9 of anyrender_vello_hybrid was measured
on top of it.

## Updating

Do not edit anything else here, and do not run `cargo fmt` on the crate (the
patched files were rustfmt-clean upstream, and only the patch was formatted).
To move to a new version, replace the directory with that version's source,
add `render_copy.rs` and `gpu_log.rs` back and reapply the hooks (search for
`svelte-blitz patch`).
