//! Render a document to a PNG on the CPU. No GPU, window or display required,
//! which makes it usable in CI and for visual regression tests.

use anyrender::render_to_buffer;
use anyrender_vello_cpu::VelloCpuImageRenderer;
use blitz_traits::shell::{ColorScheme, Viewport};

use crate::SvelteDocument;

/// Lay the document out at `width` x `height` CSS pixels and rasterise it.
/// Returns RGBA8 pixels plus the physical size.
pub fn render_rgba(
    doc: &mut SvelteDocument,
    width: u32,
    height: u32,
    scale: f32,
) -> (Vec<u8>, u32, u32) {
    let physical_width = (width as f32 * scale).round() as u32;
    let physical_height = (height as f32 * scale).round() as u32;

    let time = doc.headless_time;
    let base = doc.base_mut();
    base.set_viewport(Viewport::new(
        physical_width,
        physical_height,
        scale,
        ColorScheme::Light,
    ));
    base.resolve(time);

    let pixels = render_to_buffer::<VelloCpuImageRenderer, _>(
        |scene| {
            blitz_paint::paint_scene(
                scene,
                base,
                scale as f64,
                physical_width,
                physical_height,
                0,
                0,
            )
        },
        physical_width,
        physical_height,
    );

    (pixels, physical_width, physical_height)
}

/// Like [`render_rgba`], encoded as a PNG file.
pub fn render_png(doc: &mut SvelteDocument, width: u32, height: u32, scale: f32) -> Vec<u8> {
    let (pixels, w, h) = render_rgba(doc, width, height, scale);
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, w, h);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("png header");
        writer.write_image_data(&pixels).expect("png data");
    }
    out
}
