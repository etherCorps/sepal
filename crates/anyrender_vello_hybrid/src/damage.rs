//! svelte-blitz patch: repaint only what changed.
//!
//! Each frame's drawing commands are recorded, compared with the last frame's, and
//! only the region whose commands differ is drawn again, over the last frame kept in
//! a texture of its own (`window_renderer`, `Kept`). The comparison needs no knowledge of the
//! document: whatever changes pixels (a hover, a caret, an animation, a loaded image)
//! changes a command. Commands are matched in order (Myers' diff), so a change in one
//! place does not mark everything painted after it.

use anyrender::recording::RenderCommand;
use anyrender::{Paint, PaintRef, PaintScene};
use kurbo::{Affine, Rect, Shape};
use std::sync::OnceLock;

/// More edits than this between two frames: repaint everything.
const MAX_EDITS: usize = 500;
/// Damage covering more than this share of the window: repaint everything.
const MAX_SHARE: f64 = 0.5;
/// Damage is grown to multiples of these: vello_common's wide tile (256 x 4).
const ALIGN_X: f64 = 256.0;
const ALIGN_Y: f64 = 4.0;

/// Whether windows repaint only what changed: on unless `SVELTE_BLITZ_DAMAGE=0`.
pub(crate) fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("SVELTE_BLITZ_DAMAGE").as_deref() != Ok("0"))
}

/// `SVELTE_BLITZ_DAMAGE=verify`: after every frame that was not drawn in full,
/// also draw it in full and compare (`window_renderer`, `verify_frame`). Slow;
/// for checking, never for measuring.
pub(crate) fn verify() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("SVELTE_BLITZ_DAMAGE").as_deref() == Ok("verify"))
}

/// `SVELTE_BLITZ_TRACE=1` prints each frame's damage.
pub(crate) fn trace() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("SVELTE_BLITZ_TRACE").is_some())
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Damage {
    /// Nothing changed on screen.
    None,
    /// Repaint the whole target.
    Full,
    /// Repaint this region, in device pixels, aligned and inside the target.
    Rect(Rect),
}

#[derive(Default)]
pub(crate) struct DamageTracker {
    commands: Vec<RenderCommand>,
    bounds: Vec<Rect>,
    size: (u32, u32),
    valid: bool,
}

impl DamageTracker {
    /// The next frame repaints everything: the target was resized or recreated.
    pub(crate) fn invalidate(&mut self) {
        self.valid = false;
    }

    /// Compares `commands` with the last frame's, keeps them for the next frame, and
    /// returns what has to be repainted. `bounds` (one per command) is filled in.
    pub(crate) fn update(&mut self, commands: Vec<RenderCommand>, size: (u32, u32)) -> Damage {
        let bounds = visible_bounds(&commands);
        let damage = if !self.valid || self.size != size {
            if trace() {
                eprintln!("[trace] damage: full repaint (first frame, or the target is new)");
            }
            Damage::Full
        } else {
            self.diff(&commands, &bounds, size)
        };
        self.commands = commands;
        self.bounds = bounds;
        self.size = size;
        self.valid = true;
        damage
    }

    /// The commands of the frame just recorded, and their visible bounds.
    pub(crate) fn current(&self) -> (&[RenderCommand], &[Rect]) {
        (&self.commands, &self.bounds)
    }

    fn diff(&self, commands: &[RenderCommand], bounds: &[Rect], size: (u32, u32)) -> Damage {
        let mut region = Rect::ZERO;
        let mut add = |r: Rect| {
            if r.area() > 0.0 {
                region = if region.area() > 0.0 { region.union(r) } else { r };
            }
        };
        // A texture registered by the app can change without its command changing.
        for (command, r) in commands.iter().zip(bounds) {
            if uses_resource(command) {
                add(*r);
            }
        }
        // Commands that cannot draw (empty box: clipped away, or NaN geometry, which
        // never compares equal) are left out of the comparison.
        let drawing = |commands: &[RenderCommand], bounds: &[Rect]| -> Vec<usize> {
            (0..commands.len()).filter(|&i| bounds[i].area() > 0.0).collect()
        };
        let (old_at, new_at) = (drawing(&self.commands, &self.bounds), drawing(commands, bounds));
        let old: Vec<&RenderCommand> = old_at.iter().map(|&i| &self.commands[i]).collect();
        let new: Vec<&RenderCommand> = new_at.iter().map(|&i| &commands[i]).collect();
        let Some((unmatched_old, unmatched_new)) = unmatched(&old, &new, MAX_EDITS) else {
            if trace() {
                eprintln!("[trace] damage: full repaint (over {MAX_EDITS} commands changed of {})", new.len());
            }
            return Damage::Full;
        };
        if trace() {
            eprintln!(
                "[trace] damage: {} of {} commands gone, {} new",
                unmatched_old.len(),
                old.len(),
                unmatched_new.len()
            );
        }
        for i in unmatched_old {
            add(self.bounds[old_at[i]]);
        }
        for i in unmatched_new {
            add(bounds[new_at[i]]);
        }
        let target = Rect::new(0.0, 0.0, f64::from(size.0), f64::from(size.1));
        let region = Rect::new(
            (region.x0 / ALIGN_X).floor() * ALIGN_X,
            (region.y0 / ALIGN_Y).floor() * ALIGN_Y,
            (region.x1 / ALIGN_X).ceil() * ALIGN_X,
            (region.y1 / ALIGN_Y).ceil() * ALIGN_Y,
        )
        .intersect(target);
        if region.area() <= 0.0 {
            Damage::None
        } else if region.area() > target.area() * MAX_SHARE {
            Damage::Full
        } else {
            Damage::Rect(region)
        }
    }
}

/// Draws the commands that touch `region` (with their layers), moved so that the
/// region's corner is the scene's (0, 0).
pub(crate) fn replay(
    painter: &mut impl PaintScene,
    commands: &[RenderCommand],
    bounds: &[Rect],
    region: Rect,
) {
    let to_region = Affine::translate((-region.x0, -region.y0));
    // Layers that miss the region, with everything in them, are left out.
    let mut skipped = 0usize;
    for (command, r) in commands.iter().zip(bounds) {
        let hits = r.intersect(region).area() > 0.0;
        match command {
            RenderCommand::PushLayer(c) => {
                if skipped > 0 || !hits {
                    skipped += 1;
                    continue;
                }
                painter.push_layer(
                    c.fill,
                    c.blend,
                    c.alpha,
                    to_region * c.transform,
                    &c.clip,
                    c.filter.clone(),
                    c.backdrop_filter.clone(),
                );
            }
            RenderCommand::PushClipLayer(c) => {
                if skipped > 0 || !hits {
                    skipped += 1;
                    continue;
                }
                painter.push_clip_layer(c.fill, to_region * c.transform, &c.clip);
            }
            RenderCommand::PopLayer => {
                if skipped > 0 {
                    skipped -= 1;
                } else {
                    painter.pop_layer();
                }
            }
            _ if skipped > 0 || !hits => {}
            RenderCommand::Stroke(c) => painter.stroke(
                &c.style,
                to_region * c.transform,
                brush(&c.brush),
                c.brush_transform,
                &c.shape,
            ),
            RenderCommand::Fill(c) => painter.fill(
                c.fill,
                to_region * c.transform,
                brush(&c.brush),
                c.brush_transform,
                &c.shape,
            ),
            RenderCommand::GlyphRun(c) => painter.draw_glyphs(
                &c.font_data,
                c.font_size,
                c.hint,
                &c.normalized_coords,
                c.embolden,
                &c.style,
                brush(&c.brush),
                c.brush_alpha,
                to_region * c.transform,
                c.glyph_transform,
                c.glyphs.iter().copied(),
            ),
            RenderCommand::BoxShadow(c) => painter.draw_box_shadow(
                to_region * c.transform,
                c.rect,
                c.brush,
                c.radius,
                c.std_dev,
            ),
        }
    }
}

fn brush(paint: &Paint) -> PaintRef<'_> {
    match paint {
        Paint::Solid(color) => Paint::Solid(*color),
        Paint::Gradient(gradient) => Paint::Gradient(gradient),
        Paint::Image(image) => Paint::Image(image.as_ref()),
        Paint::Resource(id) => Paint::Resource(*id),
        Paint::Custom(custom) => Paint::Custom(custom.as_ref()),
    }
}

fn uses_resource(command: &RenderCommand) -> bool {
    let brush = match command {
        RenderCommand::Fill(c) => &c.brush,
        RenderCommand::Stroke(c) => &c.brush,
        RenderCommand::GlyphRun(c) => &c.brush,
        _ => return false,
    };
    matches!(brush, Paint::Resource(_))
}

/// Indices of the commands of `old` and of `new` that the other does not share, in
/// order (Myers' diff), or `None` for more than `max_edits` edits.
pub(crate) fn unmatched<T: PartialEq>(
    old: &[T],
    new: &[T],
    max_edits: usize,
) -> Option<(Vec<usize>, Vec<usize>)> {
    // Common start and end first: usually all but a few commands.
    let prefix = old.iter().zip(new).take_while(|(a, b)| a == b).count();
    let suffix = old[prefix..]
        .iter()
        .rev()
        .zip(new[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let a = &old[prefix..old.len() - suffix];
    let b = &new[prefix..new.len() - suffix];
    let (n, m) = (a.len() as isize, b.len() as isize);
    let max = (n + m).min(max_edits as isize);
    let offset = max + 1;
    let mut v = vec![0isize; 2 * offset as usize + 1];
    let mut trace: Vec<Vec<isize>> = Vec::new();
    let mut end = None;
    'search: for d in 0..=max {
        trace.push(v.clone());
        let mut k = -d;
        while k <= d {
            let i = (k + offset) as usize;
            let mut x = if k == -d || (k != d && v[i - 1] < v[i + 1]) {
                v[i + 1]
            } else {
                v[i - 1] + 1
            };
            let mut y = x - k;
            while x < n && y < m && a[x as usize] == b[y as usize] {
                x += 1;
                y += 1;
            }
            v[i] = x;
            if x >= n && y >= m {
                end = Some(d);
                break 'search;
            }
            k += 2;
        }
    }
    let d_end = end?;
    // Walk back through the trace: every step off the diagonal is an edit.
    let (mut gone, mut added) = (Vec::new(), Vec::new());
    let (mut x, mut y) = (n, m);
    for d in (1..=d_end).rev() {
        let v = &trace[d as usize];
        let k = x - y;
        let i = (k + offset) as usize;
        let down = k == -d || (k != d && v[i - 1] < v[i + 1]);
        let prev_k = if down { k + 1 } else { k - 1 };
        let prev_x = v[(prev_k + offset) as usize];
        let prev_y = prev_x - prev_k;
        // Back along the diagonal (matches), then one edit.
        while x > prev_x && y > prev_y {
            x -= 1;
            y -= 1;
        }
        if down {
            added.push(prefix + prev_y as usize);
        } else {
            gone.push(prefix + prev_x as usize);
        }
        (x, y) = (prev_x, prev_y);
    }
    Some((gone, added))
}

/// Where each command can change pixels, in device space: its shape's box (grown
/// for strokes, glyph extents and blur), transformed, and cut to the layers it is in.
/// A layer's own commands cover its clip.
pub(crate) fn visible_bounds(commands: &[RenderCommand]) -> Vec<Rect> {
    let everywhere = Rect::new(-1e9, -1e9, 1e9, 1e9);
    let mut clips: Vec<Rect> = Vec::new();
    let mut out = Vec::with_capacity(commands.len());
    for command in commands {
        let clip = *clips.last().unwrap_or(&everywhere);
        let r = match command {
            RenderCommand::PushLayer(c) => {
                let r = bbox(c.transform, c.clip.bounding_box()).intersect(clip);
                clips.push(r);
                r
            }
            RenderCommand::PushClipLayer(c) => {
                let r = bbox(c.transform, c.clip.bounding_box()).intersect(clip);
                clips.push(r);
                r
            }
            RenderCommand::PopLayer => clips.pop().unwrap_or(Rect::ZERO),
            RenderCommand::Fill(c) => bbox(c.transform, c.shape.bounding_box()).intersect(clip),
            RenderCommand::Stroke(c) => {
                // Miter joins reach out up to half the width times the miter limit.
                let grow = c.style.width * 0.5 * c.style.miter_limit.max(1.0) + 1.0;
                bbox(c.transform, c.shape.bounding_box().inflate(grow, grow)).intersect(clip)
            }
            RenderCommand::GlyphRun(c) => {
                let size = f64::from(c.font_size).max(1.0);
                let mut local = Rect::ZERO;
                for (i, g) in c.glyphs.iter().enumerate() {
                    let (x, y) = (f64::from(g.x), f64::from(g.y));
                    // A glyph can reach well past its pen position and above the
                    // baseline; generous, as damage only needs to cover it.
                    let glyph = Rect::new(x - size, y - 1.5 * size, x + 2.0 * size, y + size);
                    local = if i == 0 { glyph } else { local.union(glyph) };
                }
                let grow = c.embolden.x.abs().max(c.embolden.y.abs())
                    + if c.glyph_transform.is_some() { size } else { 0.0 };
                bbox(c.transform, local.inflate(grow, grow)).intersect(clip)
            }
            RenderCommand::BoxShadow(c) => {
                let grow = 3.0 * c.std_dev + 1.0;
                bbox(c.transform, c.rect.inflate(grow, grow)).intersect(clip)
            }
        };
        out.push(if r.area() > 0.0 { r } else { Rect::ZERO });
    }
    out
}

/// Empty for geometry that is not finite: blitz-paint emits border paths whose
/// points are all NaN (they draw nothing), and a NaN box intersected with a clip
/// would come out as the whole clip.
fn bbox(transform: Affine, r: Rect) -> Rect {
    let r = transform.transform_rect_bbox(r);
    if r.is_finite() {
        r.inflate(1.0, 1.0)
    } else {
        Rect::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::unmatched;

    #[test]
    fn diff_finds_only_the_edits() {
        let old = [1, 2, 3, 4, 5, 6, 7, 8];
        let new = [1, 2, 30, 4, 5, 6, 70, 8, 9];
        let (gone, added) = unmatched(&old, &new, 100).unwrap();
        assert_eq!(sorted(gone), vec![2, 6]);
        assert_eq!(sorted(added), vec![2, 6, 8]);
        assert_eq!(unmatched(&old, &old, 100).unwrap(), (vec![], vec![]));
        // Inserting in front does not mark what follows.
        let (gone, added) = unmatched(&old, &[0, 1, 2, 3, 4, 5, 6, 7, 8], 100).unwrap();
        assert_eq!((gone, added), (vec![], vec![0]));
        assert!(unmatched(&[1, 2, 3], &[4, 5, 6], 2).is_none());
    }

    fn sorted(mut v: Vec<usize>) -> Vec<usize> {
        v.sort();
        v
    }
}
