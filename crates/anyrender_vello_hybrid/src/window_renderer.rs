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

struct ActiveRenderState {
    renderer: VelloHybridRenderer,
    resources: Resources,
    depth_texture_view: TextureView,
    texture_bindings: FxHashMap<ResourceId, TextureView>,
    /// svelte-blitz patch: images too large for an atlas page (`scene.rs`). Per
    /// device, so they go with it.
    own_textures: FxHashMap<u64, crate::scene::OwnTexture>,
    render_surface: SurfaceRenderer<'static>,
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

// svelte-blitz patch (layers): vello_gpu caps the textures it draws layers into at
// 4096 px, and a layer as large as the window (a Dialog's dimmed backdrop, any
// clip or opacity over the whole page) is wider than that on a 5K display at
// 2x: the frame fails and `render` below panics. `u16::MAX` lets vello_gpu clamp
// the cap to the device's own limit instead (`MemorySettings::normalize`).
//
// svelte-blitz patch (atlas): the atlas holding glyphs and images defaults to
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
    /// svelte-blitz patch (damage): the window's size (the scene's is the damaged
    /// region's), the last frame's commands, and the recording reused each frame.
    size: (u32, u32),
    damage: Option<crate::damage::DamageTracker>,
    recording: anyrender::Scene,
    #[cfg(not(target_arch = "wasm32"))]
    last_frame: Option<std::time::Instant>,
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
            #[cfg(not(target_arch = "wasm32"))]
            last_frame: None,
        }
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
        self.size = (width, height); // svelte-blitz patch (damage)

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
        let existing_device_handle = self
            .wgpu_context
            .find_compatible_device_handle(Some(&surface));

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

            // svelte-blitz patch (damage): frames are drawn into an intermediate
            // texture, which keeps the last frame for the next one to draw over
            // (`damage.rs`), and copied to the surface.
            let intermediate_texture = intermediate_texture.or_else(|| {
                crate::damage::enabled().then(|| wgpu_context::TextureConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    format: DEFAULT_TEXTURE_FORMAT,
                    alpha_conversion: None,
                })
            });

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

            let (renderer, resources) = VelloHybridRenderer::new_with(
                render_surface.device(),
                &RenderTargetConfig {
                    format: DEFAULT_TEXTURE_FORMAT,
                    width: width as u16,
                    height: height as u16,
                },
                render_settings,
            );
            let depth_texture_view = VelloHybridRenderer::create_depth_texture_view(
                render_surface.device(),
                &RenderSize {
                    width: width as u16,
                    height: height as u16,
                },
            );

            let _ = sender.send(InitOutput {
                active: ActiveRenderState {
                    renderer,
                    resources,
                    depth_texture_view,
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
                    // svelte-blitz patch (damage): a new surface, nothing kept.
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
        // svelte-blitz patch (damage): compared with the window's size, as the
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
                active.depth_texture_view = VelloHybridRenderer::create_depth_texture_view(
                    active.render_surface.device(),
                    &RenderSize {
                        width: width as u16,
                        height: height as u16,
                    },
                );
            };
        }
    }

    fn render<F: FnOnce(&mut Self::ScenePainter<'_>)>(&mut self, draw_fn: F) {
        let RenderState::Active(state) = &mut self.render_state else {
            return;
        };

        let render_surface = &mut state.render_surface;
        let (width, height) = (render_surface.config.width, render_surface.config.height);

        debug_timer!(timer, feature = "log_frame_times");

        let mut encoder =
            render_surface
                .device()
                .create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("Render scene"),
                });

        // svelte-blitz patch (damage): with damage tracking on, the frame is recorded
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
        if let (Some(tracker), Some(mut recording)) = (self.damage.as_mut(), recording) {
            let commands = std::mem::take(&mut recording.commands);
            self.recording = recording;
            let damage = tracker.update(commands, (width, height));
            if crate::damage::trace() {
                eprintln!("[trace] damage: {damage:?}");
            }
            let region = match damage {
                // Nothing on screen changed: the window keeps the last frame. Presenting
                // is what paced the redraw loop to the display (waiting for a drawable),
                // and something animating without changing pixels asks for a redraw
                // after every frame, so a skipped frame waits out a 60 Hz frame instead.
                crate::damage::Damage::None => {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        const FRAME: std::time::Duration = std::time::Duration::from_micros(16_667);
                        if let Some(last) = self.last_frame {
                            std::thread::sleep(FRAME.saturating_sub(last.elapsed()));
                        }
                        self.last_frame = Some(std::time::Instant::now());
                    }
                    return;
                }
                crate::damage::Damage::Full => kurbo::Rect::new(0., 0., width as f64, height as f64),
                crate::damage::Damage::Rect(region) => {
                    target_init = TargetInit::SrcOver;
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
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.last_frame = Some(std::time::Instant::now());
        }

        let Ok(texture_view) = render_surface.target_texture_view() else {
            // Skip frame in case of error getting surface texture
            render_surface.clear_surface_texture();
            self.scene.reset();
            if let Some(damage) = &mut self.damage {
                damage.invalidate();
            }
            return;
        };

        // Construct Vello Hybrid TextureBindings
        let mut texture_bindings = TextureBindings::new();
        for (resource_id, texture_view) in state.texture_bindings.iter() {
            texture_bindings.insert(TextureId(resource_id.into_ffi()), texture_view.clone());
        }
        for own in state.own_textures.values() {
            texture_bindings.insert(own.id, own.view.clone()); // svelte-blitz patch
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
                // svelte-blitz patch (damage): a damaged region is small enough that the
                // depth buffer's saving (less overdraw) is worth less than clearing and
                // storing a window-sized depth texture.
                matches!(target_init, TargetInit::Clear(_)).then_some(&state.depth_texture_view),
                &texture_bindings,
                target_init,
            )
            .expect("failed to render to texture");
        render_surface.queue().submit([encoder.finish()]);
        timer.record_time("render");

        drop(texture_view);

        if render_surface.maybe_blit_and_present().is_err() {
            self.scene.reset();
            return;
        }
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
