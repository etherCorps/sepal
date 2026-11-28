use anyrender::{
    PaintScene, RegisterResourceErrorKind, RenderContext, ResourceId, WindowHandle, WindowRenderer,
};
use debug_timer::debug_timer;
use futures_channel::oneshot;
use peniko::Color;
use rustc_hash::FxHashMap;
use std::future::Future;
use std::sync::Arc;
use vello_common::{TextureId, paint::ImageId};
use vello_gpu::{
    ClearSettings, RenderSettings, RenderSize, RenderTargetConfig, Renderer as VelloHybridRenderer,
    Resources, Scene as VelloHybridScene, TargetInit, TextureBindings,
};
use wgpu::{
    CommandEncoderDescriptor, CompositeAlphaMode, Features, Limits, PresentMode, Texture,
    TextureFormat, TextureView, TextureViewDescriptor,
};
use wgpu_context::{DeviceHandle, SurfaceRenderer, SurfaceRendererConfiguration, WGPUContext};

use crate::{VelloHybridScenePainter, scene::ImageManager};

/// Drive the wgpu init future. On wasm32 we spawn it onto the JS microtask
/// queue (blocking is not allowed). On native we drive it inline with
/// `pollster::block_on` — there's no ambient async runtime to spawn onto, and
/// `on_ready` then fires before `resume` returns.
#[cfg(target_arch = "wasm32")]
fn spawn_init<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_init<F: Future<Output = ()>>(f: F) {
    pollster::block_on(f);
}

/// sepal patch: the window-sized depth buffer vello_gpu draws opaque
/// strips against (Depth32Float on Apple GPUs: 15 MB at 2400x1600). It has to
/// outlive single passes (opaque strips are drawn in a pass of their own, and
/// later passes test against them), so it cannot be memoryless.
/// `SEPAL_DEPTH=0` draws without it, in paint order.
fn depth_texture(device: &wgpu::Device, width: u32, height: u32) -> Option<TextureView> {
    if std::env::var_os("SEPAL_DEPTH").is_some_and(|v| v == "0") {
        return None;
    }
    Some(VelloHybridRenderer::create_depth_texture_view(
        device,
        &RenderSize {
            width: width as u16,
            height: height as u16,
        },
    ))
}

/// sepal patch (damage): draws `region` of the recorded frame (in device
/// pixels) into `target` over what is there, or, with `full` (a texture and its
/// depth buffer), all of it into that texture instead.
#[allow(clippy::too_many_arguments)]
fn draw_region(
    state: &mut ActiveRenderState,
    scene: &mut VelloHybridScene,
    cache: &mut FxHashMap<u64, ImageId>,
    atlas_page: (u16, u16),
    commands: &[anyrender::recording::RenderCommand],
    bounds: &[kurbo::Rect],
    region: kurbo::Rect,
    target: &TextureView,
    bindings: &TextureBindings,
    full: Option<&TextureView>,
) {
    let device = state.render_surface.device().clone();
    let queue = state.render_surface.queue().clone();
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("Render region"),
    });
    scene.reset_and_resize(region.width() as u16, region.height() as u16);
    {
        let image_manager = ImageManager {
            renderer: &mut state.renderer,
            resources: &mut state.resources,
            device: &device,
            queue: &queue,
            encoder: &mut encoder,
            cache,
            own_textures: &mut state.own_textures,
            atlas_page,
        };
        let mut painter = VelloHybridScenePainter {
            recorder: None,
            scene,
            layer_stack: Vec::new(),
            image_manager,
            texture_bindings: &mut state.texture_bindings,
            device_handle: &state.render_surface.device_handle,
        };
        crate::damage::replay(&mut painter, commands, bounds, region);
    }
    state
        .renderer
        .set_root_offset([region.x0 as i32, region.y0 as i32]);
    let size = RenderSize {
        width: state.render_surface.config.width as u16,
        height: state.render_surface.config.height as u16,
    };
    state
        .renderer
        .render(
            scene,
            &mut state.resources,
            &device,
            &queue,
            &mut encoder,
            &size,
            full.unwrap_or(target),
            full.and(state.depth_texture_view.as_ref()),
            bindings,
            if full.is_some() {
                TargetInit::Clear(ClearSettings::default())
            } else {
                TargetInit::SrcOver
            },
        )
        .expect("failed to render to texture");
    queue.submit([encoder.finish()]);
    scene.reset();
}

/// sepal patch (damage, `SEPAL_DAMAGE=verify`): draws the frame just
/// drawn in part again in full, into a texture of its own, reads both back and
/// prints where they differ by more than 2/255, with the commands drawn there.
#[allow(clippy::too_many_arguments)]
fn verify_frame(
    state: &mut ActiveRenderState,
    scene: &mut VelloHybridScene,
    cache: &mut FxHashMap<u64, ImageId>,
    atlas_page: (u16, u16),
    commands: &[anyrender::recording::RenderCommand],
    bounds: &[kurbo::Rect],
    kept: &TextureView,
    bindings: &TextureBindings,
) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static FRAME: AtomicU64 = AtomicU64::new(0);
    let frame = FRAME.fetch_add(1, Ordering::Relaxed);
    let device = state.render_surface.device().clone();
    let queue = state.render_surface.queue().clone();
    let (width, height) = (state.render_surface.config.width, state.render_surface.config.height);
    let full = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("verify: the frame in full"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEFAULT_TEXTURE_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let full_view = full.create_view(&TextureViewDescriptor::default());
    let everything = kurbo::Rect::new(0.0, 0.0, f64::from(width), f64::from(height));
    draw_region(
        state, scene, cache, atlas_page, commands, bounds, everything, kept, bindings,
        Some(&full_view),
    );
    let row = width as usize * 4;
    let padded = row.div_ceil(256) * 256;
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("verify: read back"),
    });
    let read = |texture: &Texture, encoder: &mut wgpu::CommandEncoder| {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("verify: read back"),
            size: (padded * height as usize) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded as u32),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        buffer
    };
    let (a, b) = (read(kept.texture(), &mut encoder), read(&full, &mut encoder));
    queue.submit([encoder.finish()]);
    for buffer in [&a, &b] {
        buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    }
    if device.poll(wgpu::PollType::wait_indefinitely()).is_err() {
        eprintln!("[verify] frame {frame}: could not read the frame back");
        return;
    }
    let (Ok(a), Ok(b)) = (a.slice(..).get_mapped_range(), b.slice(..).get_mapped_range()) else {
        eprintln!("[verify] frame {frame}: could not read the frame back");
        return;
    };
    let (mut count, mut worst, mut first) = (0usize, 0u8, None);
    let mut area = kurbo::Rect::ZERO;
    for y in 0..height as usize {
        for x in 0..width as usize {
            let at = y * padded + x * 4;
            let d = (0..3).map(|c| a[at + c].abs_diff(b[at + c])).max().unwrap();
            worst = worst.max(d);
            if d > 2 {
                count += 1;
                let px = kurbo::Rect::new(x as f64, y as f64, x as f64 + 1.0, y as f64 + 1.0);
                area = if count == 1 { px } else { area.union(px) };
                first.get_or_insert((x, y));
            }
        }
    }
    if count == 0 {
        eprintln!("[verify] frame {frame}: same as a full repaint (largest difference {worst}/255)");
        return;
    }
    eprintln!(
        "[verify] frame {frame}: DIFFERS from a full repaint: {count} px by more than 2/255 (up to {worst}/255), within {area:?}, first at {first:?}"
    );
    for (k, (command, r)) in commands.iter().zip(bounds).enumerate() {
        if r.intersect(area).area() > 0.0 {
            let name = format!("{command:?}");
            let name: String = name.chars().take_while(|c| c.is_alphanumeric()).collect();
            eprintln!("[verify]   command {k}: {name} at {r:?}");
        }
    }
}

struct ActiveRenderState {
    renderer: VelloHybridRenderer,
    resources: Resources,
    /// sepal patch: `None` with `SEPAL_DEPTH=0` (`depth_texture`).
    depth_texture_view: Option<TextureView>,
    texture_bindings: FxHashMap<ResourceId, TextureView>,
    /// sepal patch: images too large for an atlas page (`scene.rs`). Per
    /// device, so they go with it.
    own_textures: FxHashMap<u64, crate::scene::OwnTexture>,
    /// sepal patch (damage): the last frame, kept for the next one to draw
    /// over, and how it gets to the surface (`SurfaceCopy`).
    /// The kept frame is `None` while let go when idle (`release_if_idle`).
    kept: Option<(SurfaceCopy, Option<Kept>)>,
    render_surface: SurfaceRenderer<'static>,
}

/// sepal patch (damage): copies the kept frame to the surface texture in
/// the frame's own submit, in a pass that clears the surface texture rather
/// than loads it. wgpu_context's intermediate texture did it with wgpu's
/// `TextureBlitter`, which loads, so wgpu first cleared every new surface
/// texture in a pass of its own (a full-window clear per frame), and in a
/// submit of its own.
struct SurfaceCopy {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}

/// sepal patch (damage): the kept frame, and its binding for `SurfaceCopy`.
struct Kept {
    view: TextureView,
    group: wgpu::BindGroup,
}

const SURFACE_COPY_SHADER: &str = r"
@group(0) @binding(0) var src: texture_2d<f32>;

@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    return textureLoad(src, vec2<i32>(floor(pos.xy)), 0);
}
";

impl SurfaceCopy {
    fn new(device: &wgpu::Device, format: TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sepal surface copy"),
            source: wgpu::ShaderSource::Wgsl(SURFACE_COPY_SHADER.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sepal surface copy"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sepal surface copy"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sepal surface copy"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        Self { pipeline, layout }
    }

    /// The kept frame for a window of this size.
    fn kept(&self, device: &wgpu::Device, width: u32, height: u32) -> Kept {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sepal kept frame"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEFAULT_TEXTURE_FORMAT,
            // Verify mode reads it back.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | if crate::damage::verify() {
                    wgpu::TextureUsages::COPY_SRC
                } else {
                    wgpu::TextureUsages::empty()
                },
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sepal kept frame"),
            layout: &self.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            }],
        });
        Kept { view, group }
    }

    fn encode(&self, encoder: &mut wgpu::CommandEncoder, kept: &Kept, target: &TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sepal surface copy"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &kept.group, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// Result of a successful asynchronous resume; both the active state and the
/// `WGPUContext` are returned so the renderer can reclaim the context.
struct InitOutput {
    active: ActiveRenderState,
}

#[allow(clippy::large_enum_variant)]
enum RenderState {
    Suspended,
    Pending {
        receiver: oneshot::Receiver<InitOutput>,
    },
    Active(ActiveRenderState),
}

#[derive(Clone)]
#[non_exhaustive]
pub struct VelloHybridRendererOptions {
    pub features: Option<Features>,
    pub limits: Option<Limits>,
    pub render_settings: RenderSettings,
    pub base_color: Color,
    /// Alpha mode used when compositing the window surface.
    pub composite_alpha_mode: anyrender::CompositeAlphaMode,
    /// Maximum number of frames the presentation engine may queue ahead of the
    /// display. Each queued frame requires a window-sized swapchain surface, so
    /// lower values reduce memory usage and input latency at the cost of less
    /// buffering to absorb slow frames.
    pub desired_maximum_frame_latency: u32,
}

// sepal patch (layers): vello_gpu caps the textures it draws layers into at
// 4096 px, and a layer as large as the window (a Dialog's dimmed backdrop, any
// clip or opacity over the whole page) is wider than that on a 5K display at
// 2x: the frame fails and `render` below panics. `u16::MAX` lets vello_gpu clamp
// the cap to the device's own limit instead (`MemorySettings::normalize`).
//
// sepal patch (atlas): the atlas holding glyphs and images defaults to
// pages of 4096 x 4096, 64 MB each, the first one allocated with the first
// cached glyph: most of it empty, as a page of text needs a few hundred glyphs.
// Pages of 1024 x 1024 (4 MB) are added as they fill; 128 of them hold as much
// as the default's 8. Images larger than a page get a texture of their own
// (`ImageManager::image_source` in `scene.rs`).
fn render_settings() -> RenderSettings {
    let mut settings = RenderSettings::default();
    settings.memory_settings.layers_config.max_texture_size =
        vello_common::geometry::SizeU16::new(u16::MAX);
    let atlas = &mut settings.memory_settings.image_atlas_config;
    atlas.atlas_size = (1024, 1024);
    atlas.max_atlases = 128;
    settings
}

impl Default for VelloHybridRendererOptions {
    fn default() -> Self {
        Self {
            features: None,
            limits: None,
            render_settings: render_settings(),
            base_color: Color::WHITE,
            composite_alpha_mode: anyrender::CompositeAlphaMode::Auto,
            desired_maximum_frame_latency: 1,
        }
    }
}

impl VelloHybridRendererOptions {
    pub fn new() -> Self {
        // Within default of RenderSettings there are calls to non const methods so no const for new
        Self::default()
    }

    pub const fn features(self, features: Features) -> Self {
        Self {
            features: Some(features),
            ..self
        }
    }

    pub const fn limits(self, limits: Limits) -> Self {
        Self {
            limits: Some(limits),
            ..self
        }
    }

    pub const fn render_settings(self, render_settings: RenderSettings) -> Self {
        Self {
            render_settings,
            ..self
        }
    }

    pub const fn base_color(self, base_color: Color) -> Self {
        Self { base_color, ..self }
    }

    pub const fn composite_alpha_mode(
        self,
        composite_alpha_mode: anyrender::CompositeAlphaMode,
    ) -> Self {
        Self {
            composite_alpha_mode,
            ..self
        }
    }

    pub const fn desired_maximum_frame_latency(self, desired_maximum_frame_latency: u32) -> Self {
        Self {
            desired_maximum_frame_latency,
            ..self
        }
    }
}

impl From<anyrender::RendererConfig> for VelloHybridRendererOptions {
    fn from(config: anyrender::RendererConfig) -> Self {
        Self {
            base_color: config.base_color.unwrap_or(Color::WHITE),
            composite_alpha_mode: config.composite_alpha_mode.unwrap_or_default(),
            ..Default::default()
        }
    }
}

pub struct VelloHybridWindowRenderer {
    // The fields MUST be in this order, so that the surface is dropped before the window
    // Window is cached even when suspended so that it can be reused when the app is resumed after being suspended
    render_state: RenderState,
    window_handle: Option<Arc<dyn WindowHandle>>,

    wgpu_context: WGPUContext,
    scene: VelloHybridScene,
    config: VelloHybridRendererOptions,
    cached_images: FxHashMap<u64, ImageId>,
    /// sepal patch (damage): the window's size (the scene's is the damaged
    /// region's), the last frame's commands, and the recording reused each frame.
    size: (u32, u32),
    damage: Option<crate::damage::DamageTracker>,
    recording: anyrender::Scene,
    /// The last `render` drew nothing: nothing on screen had changed.
    skipped: bool,
    /// sepal patch (idle): when a frame was last presented, and whether
    /// the kept frame and the depth buffer were let go since (`release_if_idle`).
    presented_at: Option<std::time::Instant>,
    released: bool,
    /// sepal patch (start-up): a device and the renderer made on a thread
    /// before there is a window (`prepare`), for the first `resume`.
    prepared: Option<std::thread::JoinHandle<Option<Prepared>>>,
}

/// sepal patch (start-up): what `prepare` makes ahead of the window.
struct Prepared {
    device_handle: DeviceHandle,
    renderer: VelloHybridRenderer,
    resources: Resources,
}

impl VelloHybridWindowRenderer {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self::with_options(VelloHybridRendererOptions::default())
    }

    pub fn with_options(config: impl Into<VelloHybridRendererOptions>) -> Self {
        let config = config.into();
        let render_settings = config.render_settings;
        let wgpu_context = build_wgpu_context(&config);
        Self {
            render_state: RenderState::Suspended,
            config,
            wgpu_context,
            window_handle: None,
            scene: VelloHybridScene::new_with(0, 0, render_settings.level),
            cached_images: FxHashMap::default(),
            size: (0, 0),
            damage: crate::damage::enabled().then(Default::default),
            recording: anyrender::Scene::new(),
            skipped: false,
            presented_at: None,
            released: false,
            prepared: None,
        }
    }

    /// sepal patch (start-up): make the device and the renderer's pipelines
    /// on a thread now, while the app starts, instead of on the UI thread when
    /// the window's surface exists. Only the surface needs the window; the
    /// first `resume` makes it, takes what was made here if the device
    /// supports it, and makes its own otherwise. The size is the first
    /// frame's to set (`maybe_update_config_buffer`).
    pub fn prepare(&mut self) {
        let instance = self.wgpu_context.instance.clone();
        let features = self.wgpu_context.extra_features();
        let limits = self.wgpu_context.override_limits();
        let settings = self.config.render_settings;
        self.prepared = std::thread::Builder::new()
            .name("sepal-gpu".into())
            .spawn(move || {
                let device_handle = pollster::block_on(DeviceHandle::new_from_compatible_surface(
                    instance, None, features, limits,
                ))
                .ok()?;
                let target = RenderTargetConfig { format: DEFAULT_TEXTURE_FORMAT, width: 1, height: 1 };
                let (renderer, resources) =
                    VelloHybridRenderer::new_with(&device_handle.device, &target, settings);
                Some(Prepared { device_handle, renderer, resources })
            })
            .ok();
    }

    /// sepal patch (damage): whether the last frame was skipped because
    /// nothing on screen changed. Nothing was presented, so nothing waited for
    /// the display either; a caller that redraws again at once spins.
    pub fn skipped_last_frame(&self) -> bool {
        self.skipped
    }

    /// sepal patch (damage): draw all of the next frame. For when what the
    /// window shows may not be the last frame presented: macOS drops a frame
    /// presented before the window is first on screen.
    pub fn repaint_all(&mut self) {
        if let Some(damage) = &mut self.damage {
            damage.invalidate();
        }
    }

    /// sepal patch (idle): when `release_if_idle(after)` will let go of the
    /// kept frame and the depth buffer: `after` past the last frame presented, if
    /// they are still held. The caller wakes then, once per burst of frames.
    pub fn release_due(&self, after: std::time::Duration) -> Option<std::time::Instant> {
        if self.released || !matches!(self.render_state, RenderState::Active(_)) {
            return None;
        }
        self.presented_at.map(|at| at + after)
    }

    /// sepal patch (idle): if no frame was presented for `after`, let go of
    /// the kept frame and the depth buffer (15 MB each at 2400x1600); the next
    /// frame makes them again and is drawn in full. Returns whether they were
    /// let go.
    pub fn release_if_idle(&mut self, after: std::time::Duration) -> bool {
        let RenderState::Active(state) = &mut self.render_state else {
            return false;
        };
        if self.released || self.presented_at.is_none_or(|at| at.elapsed() < after) {
            return false;
        }
        state.depth_texture_view = None;
        if let Some((_, kept)) = &mut state.kept {
            *kept = None;
        }
        // wgpu frees a texture once no submission still to be cleaned up holds it.
        let _ = state.render_surface.device().poll(wgpu::PollType::Poll);
        self.released = true;
        true
    }

    pub fn current_device_handle(&self) -> Option<&DeviceHandle> {
        match &self.render_state {
            RenderState::Active(active) => Some(&active.render_surface.device_handle),
            _ => None,
        }
    }
}

fn build_wgpu_context(config: &VelloHybridRendererOptions) -> WGPUContext {
    let features =
        config.features.unwrap_or_default() | Features::CLEAR_TEXTURE | Features::PIPELINE_CACHE;
    WGPUContext::with_features_and_limits(Some(features), config.limits.clone())
}

// TODO: Make configurable?
#[cfg(target_os = "android")]
const DEFAULT_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
#[cfg(not(target_os = "android"))]
const DEFAULT_TEXTURE_FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;

impl RenderContext for VelloHybridWindowRenderer {
    fn renderer_specific_context(&self) -> Option<Box<dyn std::any::Any>> {
        match &self.render_state {
            RenderState::Active(state) => {
                Some(Box::new(state.render_surface.device_handle.clone()) as _)
            }
            RenderState::Suspended => None,
            RenderState::Pending { .. } => None,
        }
    }

    fn try_register_custom_resource(
        &mut self,
        resource: Box<dyn std::any::Any>,
    ) -> Result<anyrender::ResourceId, anyrender::RegisterResourceError> {
        let RenderState::Active(state) = &mut self.render_state else {
            return Err(RegisterResourceErrorKind::Other.into());
        };

        // Try to downcast as Texture
        match resource.downcast::<Texture>() {
            Ok(texture) => {
                let id = ResourceId::new();
                let texture_view = texture.create_view(&TextureViewDescriptor::default());
                state.texture_bindings.insert(id, texture_view);
                Ok(id)
            }
            Err(resource) => {
                // Else try to downcast as TextureView
                if let Ok(texture_view) = resource.downcast::<TextureView>() {
                    let id = ResourceId::new();
                    state.texture_bindings.insert(id, *texture_view);
                    Ok(id)
                }
                // Else return error
                else {
                    Err(anyrender::RegisterResourceErrorKind::UnsupportedResourceKind.into())
                }
            }
        }
    }

    fn unregister_resource(&mut self, resource_id: ResourceId) {
        let RenderState::Active(state) = &mut self.render_state else {
            return;
        };
        state.texture_bindings.remove(&resource_id);
    }
}

impl WindowRenderer for VelloHybridWindowRenderer {
    type ScenePainter<'a>
        = VelloHybridScenePainter<'a>
    where
        Self: 'a;

    fn is_active(&self) -> bool {
        matches!(self.render_state, RenderState::Active { .. })
    }

    fn is_pending(&self) -> bool {
        matches!(self.render_state, RenderState::Pending { .. })
    }

    fn resume<F: FnOnce() + 'static>(
        &mut self,
        window_handle: Arc<dyn WindowHandle>,
        width: u32,
        height: u32,
        on_ready: F,
    ) {
        // Each `resume` must be preceded by `suspend` (or be the first call after
        // construction). Calling while `Pending` or `Active` is a state-machine bug
        // in the embedder: it would orphan the in-flight init's `WGPUContext` and
        // pay for a fresh adapter+device init on the fallback path below.
        if !matches!(self.render_state, RenderState::Suspended) {
            // #[cfg(feature = "tracing")]
            // tracing::warn!("WindowRenderer::resume called from non-Suspended state");
            return;
        }

        let (sender, receiver) = oneshot::channel();
        self.render_state = RenderState::Pending { receiver };
        self.window_handle = Some(window_handle.clone());

        // Reset the scene to the new dimensions before init kicks off, so callers that
        // query scene size (e.g. `set_size`) see consistent state.
        let render_settings = self.config.render_settings;
        self.scene = VelloHybridScene::new_with(width as u16, height as u16, render_settings.level);
        self.size = (width, height); // sepal patch (damage)

        let surface = self
            .wgpu_context
            .create_surface(window_handle)
            .expect("Error creating surface");
        let instance = self.wgpu_context.instance.clone();
        let extra_features = self.wgpu_context.extra_features();
        let override_limits = self.wgpu_context.override_limits();
        let mut composite_alpha_mode = match self.config.composite_alpha_mode {
            anyrender::CompositeAlphaMode::Auto => CompositeAlphaMode::Auto,
            anyrender::CompositeAlphaMode::Opaque => CompositeAlphaMode::Opaque,
            anyrender::CompositeAlphaMode::Transparent => {
                #[cfg(target_vendor = "apple")]
                {
                    // wgpu is lying in apple's case it uses PreMultiplied in reality
                    // (do not modify shaders for PostMultiplied)
                    CompositeAlphaMode::PostMultiplied
                }
                #[cfg(not(target_vendor = "apple"))]
                {
                    CompositeAlphaMode::PreMultiplied
                }
            }
        };
        let desired_maximum_frame_latency = self.config.desired_maximum_frame_latency;
        // sepal patch (start-up): what `prepare` made, offered as an existing
        // device that has to support this surface; its renderer is used only
        // if that device is the one taken. `complete_resume` pools the device
        // taken, so it is only in the pool while `find` looks.
        let pool_was_empty = self.wgpu_context.device_pool.is_empty();
        let mut prepared = self.prepared.take().and_then(|thread| thread.join().ok()).flatten();
        if let Some(prepared) = &prepared {
            self.wgpu_context.device_pool.push(prepared.device_handle.clone());
        }
        let existing_device_handle = self
            .wgpu_context
            .find_compatible_device_handle(Some(&surface));
        if prepared.is_some() {
            self.wgpu_context.device_pool.pop();
            if !(pool_was_empty && existing_device_handle.is_some()) {
                prepared = None;
            }
        }

        spawn_init(async move {
            let device_handle = match existing_device_handle {
                Some(device_handle) => device_handle,
                None => DeviceHandle::new_from_compatible_surface(
                    instance,
                    Some(&surface),
                    extra_features,
                    override_limits,
                )
                .await
                .expect("Error creating DeviceHandle"),
            };

            let adapter = &device_handle.adapter;
            let caps = surface.get_capabilities(adapter);
            let mut alpha_modes = caps.alpha_modes;

            if !alpha_modes.contains(&composite_alpha_mode) {
                alpha_modes.sort_unstable_by(
                    |first: &CompositeAlphaMode, second: &CompositeAlphaMode| {
                        let first_num = match *first {
                            CompositeAlphaMode::PreMultiplied => 0,
                            CompositeAlphaMode::PostMultiplied => 1,
                            CompositeAlphaMode::Opaque
                            | CompositeAlphaMode::Inherit
                            | CompositeAlphaMode::Auto => 2,
                        };
                        let second_num = match *second {
                            CompositeAlphaMode::PreMultiplied => 0,
                            CompositeAlphaMode::PostMultiplied => 1,
                            CompositeAlphaMode::Opaque
                            | CompositeAlphaMode::Inherit
                            | CompositeAlphaMode::Auto => 2,
                        };
                        first_num.cmp(&second_num)
                    },
                );
                composite_alpha_mode = alpha_modes
                    .first()
                    .copied()
                    .expect("Surface didn't report any alpha modes");
            }

            // Vello Hybrid emits premultiplied alpha and renders directly into
            // its target. That matches a `PreMultiplied` surface (and Opaque
            // ignores alpha), so those render straight to the surface. Only
            // `PostMultiplied` (straight alpha) needs conversion, which routes
            // through an intermediate texture (using the renderer's target
            // format) that is un-premultiplied while blitting to the surface.
            #[cfg(not(target_vendor = "apple"))]
            let intermediate_texture = (composite_alpha_mode == CompositeAlphaMode::PostMultiplied)
                .then(|| {
                    use wgpu::TextureUsages;
                    use wgpu_context::{AlphaConversion, TextureConfiguration};
                    TextureConfiguration {
                        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                        format: DEFAULT_TEXTURE_FORMAT,
                        alpha_conversion: Some(AlphaConversion::Unpremultiply),
                    }
                });

            // Apple is almost guaranteed to be premultiplied
            // TODO: Remove below once gfx-rs/wgpu#9896 gets fixed
            #[cfg(target_vendor = "apple")]
            let intermediate_texture = None;

            // sepal patch (damage): frames are drawn into a texture of our
            // own, which keeps the last frame for the next one to draw over
            // (`damage.rs`), and copied to the surface (`SurfaceCopy`).

            let render_surface = SurfaceRenderer::new(
                surface,
                SurfaceRendererConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    formats: vec![DEFAULT_TEXTURE_FORMAT],
                    width,
                    height,
                    present_mode: PresentMode::AutoVsync,
                    desired_maximum_frame_latency,
                    alpha_mode: composite_alpha_mode,
                    view_formats: vec![],
                },
                intermediate_texture,
                device_handle,
            )
            .expect("Error creating SurfaceRenderer");

            let (renderer, resources) = match prepared {
                Some(Prepared { renderer, resources, .. }) => (renderer, resources),
                None => VelloHybridRenderer::new_with(
                    render_surface.device(),
                    &RenderTargetConfig {
                        format: DEFAULT_TEXTURE_FORMAT,
                        width: width as u16,
                        height: height as u16,
                    },
                    render_settings,
                ),
            };
            let depth_texture_view = depth_texture(render_surface.device(), width, height);
            let kept = crate::damage::enabled().then(|| {
                let copy = SurfaceCopy::new(render_surface.device(), render_surface.config.format);
                let kept = copy.kept(render_surface.device(), width, height);
                (copy, Some(kept))
            });

            let _ = sender.send(InitOutput {
                active: ActiveRenderState {
                    renderer,
                    resources,
                    depth_texture_view,
                    kept,
                    render_surface,
                    texture_bindings: FxHashMap::default(),
                    own_textures: FxHashMap::default(),
                },
            });
            on_ready();
        });
    }

    fn complete_resume(&mut self) -> bool {
        match &mut self.render_state {
            RenderState::Active { .. } => true,
            RenderState::Suspended => false,
            RenderState::Pending { receiver } => match receiver.try_recv() {
                Ok(Some(InitOutput { active })) => {
                    let device_handle = active.render_surface.device_handle.clone();
                    self.wgpu_context.device_pool.push(device_handle);
                    // sepal patch (damage): a new surface, nothing kept.
                    self.size = (active.render_surface.config.width, active.render_surface.config.height);
                    if let Some(damage) = &mut self.damage {
                        damage.invalidate();
                    }
                    self.render_state = RenderState::Active(active);
                    true
                }
                _ => false,
            },
        }
    }

    fn suspend(&mut self) {
        self.render_state = RenderState::Suspended;
        if let Some(damage) = &mut self.damage {
            damage.invalidate();
        }
    }

    fn set_size(&mut self, width: u32, height: u32) {
        // sepal patch (damage): compared with the window's size, as the
        // scene is resized to the damaged region every frame.
        if (width, height) != self.size {
            self.size = (width, height);
            if let Some(damage) = &mut self.damage {
                damage.invalidate();
            }
            self.scene = VelloHybridScene::new_with(
                width as u16,
                height as u16,
                self.config.render_settings.level,
            );
            if let RenderState::Active(active) = &mut self.render_state {
                active.render_surface.resize(width, height);
                active.depth_texture_view =
                    depth_texture(active.render_surface.device(), width, height);
                if let Some((copy, kept)) = &mut active.kept {
                    *kept = Some(copy.kept(active.render_surface.device(), width, height));
                }
            };
        }
    }

    fn render<F: FnOnce(&mut Self::ScenePainter<'_>)>(&mut self, draw_fn: F) {
        let RenderState::Active(state) = &mut self.render_state else {
            return;
        };

        let render_surface = &mut state.render_surface;
        let (width, height) = (render_surface.config.width, render_surface.config.height);

        // sepal patch (idle): made again after `release_if_idle`; what the
        // window shows is no longer kept anywhere, so this frame is drawn in full.
        if self.released {
            self.released = false;
            state.depth_texture_view = depth_texture(render_surface.device(), width, height);
            if let Some((copy, kept)) = &mut state.kept {
                *kept = Some(copy.kept(render_surface.device(), width, height));
            }
            if let Some(damage) = &mut self.damage {
                damage.invalidate();
            }
        }

        debug_timer!(timer, feature = "log_frame_times");

        let mut encoder =
            render_surface
                .device()
                .create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("Render scene"),
                });

        // sepal patch (damage): with damage tracking on, the frame is recorded
        // here, compared with the last one, and only what changed is drawn below.
        let mut recording = self
            .damage
            .is_some()
            .then(|| std::mem::take(&mut self.recording));
        {
            let image_manager = ImageManager {
                renderer: &mut state.renderer,
                resources: &mut state.resources,
                device: render_surface.device(),
                queue: render_surface.queue(),
                encoder: &mut encoder,
                cache: &mut self.cached_images,
                own_textures: &mut state.own_textures,
                atlas_page: self.config.render_settings.memory_settings.image_atlas_config.atlas_size,
            };

            let mut scene_painter = VelloHybridScenePainter {
                recorder: recording.as_mut(),
                scene: &mut self.scene,
                layer_stack: Vec::new(),
                image_manager,
                texture_bindings: &mut state.texture_bindings,
                device_handle: &render_surface.device_handle,
            };
            if self.config.base_color != Color::TRANSPARENT {
                scene_painter.fill(
                    peniko::Fill::NonZero,
                    kurbo::Affine::IDENTITY,
                    self.config.base_color,
                    None,
                    &kurbo::Rect::new(0., 0., width as f64, height as f64),
                );
            }
            // Regenerate the vello scene
            draw_fn(&mut scene_painter);
        }
        timer.record_time("cmd");

        let mut root_offset = [0, 0];
        let mut target_init = TargetInit::Clear(ClearSettings::default());
        // sepal patch (damage): whether to check the frame.
        let mut verify = false;
        if let (Some(tracker), Some(mut recording)) = (self.damage.as_mut(), recording) {
            let commands = std::mem::take(&mut recording.commands);
            self.recording = recording;
            let damage = tracker.update(commands, (width, height));
            if crate::damage::trace() {
                eprintln!("[trace] damage: {damage:?}");
            }
            self.skipped = matches!(damage, crate::damage::Damage::None);
            let region = match damage {
                // Nothing on screen changed: the window keeps the last frame. Presenting
                // is what paced the redraw loop to the display (it waits for a drawable),
                // so the caller has to pace the next redraw itself (`skipped_last_frame`):
                // something animating without changing pixels asks for one after every
                // frame. Never by waiting here: that holds up input on the same thread.
                crate::damage::Damage::None => return,
                crate::damage::Damage::Full => kurbo::Rect::new(0., 0., width as f64, height as f64),
                crate::damage::Damage::Rect(region) => {
                    target_init = TargetInit::SrcOver;
                    verify = crate::damage::verify();
                    region
                }
            };
            root_offset = [region.x0 as i32, region.y0 as i32];
            self.scene
                .reset_and_resize(region.width() as u16, region.height() as u16);
            let image_manager = ImageManager {
                renderer: &mut state.renderer,
                resources: &mut state.resources,
                device: render_surface.device(),
                queue: render_surface.queue(),
                encoder: &mut encoder,
                cache: &mut self.cached_images,
                own_textures: &mut state.own_textures,
                atlas_page: self.config.render_settings.memory_settings.image_atlas_config.atlas_size,
            };
            let mut scene_painter = VelloHybridScenePainter {
                recorder: None,
                scene: &mut self.scene,
                layer_stack: Vec::new(),
                image_manager,
                texture_bindings: &mut state.texture_bindings,
                device_handle: &render_surface.device_handle,
            };
            let (commands, bounds) = tracker.current();
            crate::damage::replay(&mut scene_painter, commands, bounds, region);
        }
        state.renderer.set_root_offset(root_offset);

        let Ok(surface_view) = render_surface.target_texture_view() else {
            // Skip frame in case of error getting surface texture
            render_surface.clear_surface_texture();
            self.scene.reset();
            if let Some(damage) = &mut self.damage {
                damage.invalidate();
            }
            return;
        };

        // sepal patch (damage): drawn into the kept frame, copied to the
        // surface below.
        let texture_view = match &state.kept {
            Some((_, Some(kept))) => kept.view.clone(),
            _ => surface_view.clone(),
        };

        // Construct Vello Hybrid TextureBindings
        let mut texture_bindings = TextureBindings::new();
        for (resource_id, texture_view) in state.texture_bindings.iter() {
            texture_bindings.insert(TextureId(resource_id.into_ffi()), texture_view.clone());
        }
        for own in state.own_textures.values() {
            texture_bindings.insert(own.id, own.view.clone()); // sepal patch
        }

        state
            .renderer
            .render(
                &self.scene,
                &mut state.resources,
                render_surface.device(),
                render_surface.queue(),
                &mut encoder,
                &RenderSize {
                    width: render_surface.config.width as u16,
                    height: render_surface.config.height as u16,
                },
                &texture_view,
                // sepal patch (damage): a damaged region is small enough that the
                // depth buffer's saving (less overdraw) is worth less than clearing and
                // storing a window-sized depth texture.
                state
                    .depth_texture_view
                    .as_ref()
                    .filter(|_| matches!(target_init, TargetInit::Clear(_))),
                &texture_bindings,
                target_init,
            )
            .expect("failed to render to texture");
        if let Some((copy, Some(kept))) = &state.kept {
            copy.encode(&mut encoder, kept, &surface_view); // sepal patch (damage)
        }
        render_surface.queue().submit([encoder.finish()]);
        // sepal patch (damage): check a frame not drawn in full.
        let atlas_page = self.config.render_settings.memory_settings.image_atlas_config.atlas_size;
        if let Some(tracker) = &self.damage {
            let (commands, bounds) = tracker.current();
            if verify {
                verify_frame(
                    state,
                    &mut self.scene,
                    &mut self.cached_images,
                    atlas_page,
                    commands,
                    bounds,
                    &texture_view,
                    &texture_bindings,
                );
            }
        }
        let render_surface = &mut state.render_surface;
        timer.record_time("render");

        drop(texture_view);
        drop(surface_view);

        if render_surface.maybe_blit_and_present().is_err() {
            self.scene.reset();
            return;
        }
        self.presented_at = Some(std::time::Instant::now()); // sepal patch (idle)
        timer.record_time("present");

        render_surface
            .device()
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();

        timer.record_time("wait");
        timer.print_times("vello_hybrid: ");

        // Empty the Vello scene (memory optimisation)
        self.scene.reset();
    }
}
