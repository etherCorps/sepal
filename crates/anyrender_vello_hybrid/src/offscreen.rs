//! svelte-blitz patch: this whole file. Upstream can only draw into a window.
//!
//! Draws a scene with the same renderer the window uses, into a texture, and
//! reads the pixels back. For looking at what the GPU renderer produces
//! without a window; needs a GPU, so it is no use on a CI machine.

use std::num::NonZeroU32;

use anyrender::PaintScene;
use peniko::Color;
use rustc_hash::FxHashMap;
use vello_gpu::{
    ClearSettings, RenderSize, RenderTargetConfig, Renderer, Scene, TargetInit, TextureBindings,
};
use wgpu::{
    BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d, TexelCopyBufferInfo,
    TexelCopyBufferLayout, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureViewDescriptor,
};
use wgpu_context::{DeviceHandle, WGPUContext};

use crate::{VelloHybridRendererOptions, VelloHybridScenePainter, scene::ImageManager};

/// Render what `draw` paints onto a white page of `width` × `height` pixels.
/// Returns RGBA, 8 bits per channel, row by row.
pub fn render_to_rgba(
    width: u32,
    height: u32,
    draw: impl FnOnce(&mut VelloHybridScenePainter<'_>),
) -> Result<Vec<u8>, String> {
    let options = VelloHybridRendererOptions::default();
    let features = wgpu::Features::CLEAR_TEXTURE | wgpu::Features::PIPELINE_CACHE;
    let context = WGPUContext::with_features_and_limits(Some(features), None);
    let device_handle = pollster::block_on(DeviceHandle::new_from_compatible_surface(
        context.instance.clone(),
        None,
        context.extra_features(),
        context.override_limits(),
    ))
    .map_err(|error| format!("no GPU device: {error:?}"))?;
    let (device, queue) = (&device_handle.device, &device_handle.queue);

    const FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
    let size = RenderSize {
        width: width as u16,
        height: height as u16,
    };
    let target = device.create_texture(&TextureDescriptor {
        label: Some("offscreen target"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: FORMAT,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&TextureViewDescriptor::default());
    let (mut renderer, mut resources) = Renderer::new_with(
        device,
        &RenderTargetConfig {
            format: FORMAT,
            width: size.width,
            height: size.height,
        },
        options.render_settings,
    );
    let depth = Renderer::create_depth_texture_view(device, &size);
    let mut scene = Scene::new_with(size.width, size.height, options.render_settings.level);
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("offscreen"),
    });

    let mut cached_images = FxHashMap::default();
    let mut bindings = FxHashMap::default();
    {
        let mut painter = VelloHybridScenePainter {
            scene: &mut scene,
            layer_stack: Vec::new(),
            image_manager: ImageManager {
                renderer: &mut renderer,
                resources: &mut resources,
                device,
                queue,
                encoder: &mut encoder,
                cache: &mut cached_images,
            },
            texture_bindings: &mut bindings,
            device_handle: &device_handle,
        };
        painter.fill(
            peniko::Fill::NonZero,
            kurbo::Affine::IDENTITY,
            Color::WHITE,
            None,
            &kurbo::Rect::new(0., 0., width as f64, height as f64),
        );
        draw(&mut painter);
    }

    renderer
        .render(
            &scene,
            &mut resources,
            device,
            queue,
            &mut encoder,
            &size,
            &view,
            Some(&depth),
            &TextureBindings::new(),
            TargetInit::Clear(ClearSettings::default()),
        )
        .map_err(|error| format!("rendering failed: {error:?}"))?;

    // Rows in a copy have to be padded to a multiple of 256 bytes.
    let row = width as usize * 4;
    let padded = row.div_ceil(256) * 256;
    let buffer = device.create_buffer(&BufferDescriptor {
        label: Some("offscreen readback"),
        size: (padded * height as usize) as u64,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        TexelCopyBufferInfo {
            buffer: &buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(padded as u32).map(u32::from),
                rows_per_image: None,
            },
        },
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);

    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|error| format!("waiting for the GPU failed: {error:?}"))?;
    let data = slice
        .get_mapped_range()
        .map_err(|error| format!("reading the pixels back failed: {error:?}"))?;
    let mut pixels = Vec::with_capacity(row * height as usize);
    for line in data.chunks(padded) {
        pixels.extend_from_slice(&line[..row]);
    }
    Ok(pixels)
}
