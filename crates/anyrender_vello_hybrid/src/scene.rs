use std::sync::Arc;

use anyrender::{Filter, NormalizedCoord, Paint, PaintRef, PaintScene, RenderContext, ResourceId};
use glifo::FontEmbolden;
use kurbo::{Affine, Diagonal2, Rect, Shape, Stroke};
use peniko::{BlendMode, Color, Fill, FontData, ImageBrush, ImageData, StyleRef};
use rustc_hash::FxHashMap;
use vello_common::{
    TextureId,
    geometry::RectU16,
    paint::{ImageId, ImageSource, PaintType},
};
use vello_gpu::{Renderer, Resources};
use wgpu::{CommandEncoder, Device, Queue, Texture, TextureView, TextureViewDescriptor};
use wgpu_context::DeviceHandle;

const DEFAULT_TOLERANCE: f64 = 0.1;

fn anyrender_paint_to_vello_hybrid_paint<'a>(
    paint: PaintRef<'a>,
    image_manager: &mut ImageManager<'_>,
    texture_bindings: &FxHashMap<ResourceId, TextureView>,
) -> PaintType {
    match paint {
        Paint::Solid(alpha_color) => PaintType::Solid(alpha_color),
        Paint::Gradient(gradient) => PaintType::Gradient(gradient.clone()),

        Paint::Image(image_brush) => match image_manager.image_source(image_brush.image) {
            Some(image) => PaintType::Image(ImageBrush {
                image,
                sampler: image_brush.sampler,
            }),
            None => PaintType::Solid(peniko::color::palette::css::TRANSPARENT),
        },

        Paint::Resource(brush) => match texture_bindings.get(&brush.image) {
            Some(texture_view) => {
                let texture = texture_view.texture();
                PaintType::Image(ImageBrush {
                    image: ImageSource::external_texture(
                        TextureId(brush.image.into_ffi()),
                        RectU16 {
                            x0: 0,
                            y0: 0,
                            x1: texture.width().min(u16::MAX as u32) as u16,
                            y1: texture.height().min(u16::MAX as u32) as u16,
                        },
                        true,
                    ),
                    sampler: brush.sampler,
                })
            }
            None => PaintType::Solid(peniko::color::palette::css::TRANSPARENT),
        },

        // TODO: custom paint
        Paint::Custom(_) => PaintType::Solid(peniko::color::palette::css::TRANSPARENT),
    }
}

pub struct ImageManager<'a> {
    pub(crate) renderer: &'a mut Renderer,
    pub(crate) resources: &'a mut Resources,
    pub(crate) device: &'a Device,
    pub(crate) queue: &'a Queue,
    pub(crate) encoder: &'a mut CommandEncoder,
    pub(crate) cache: &'a mut FxHashMap<u64, ImageId>,
    /// svelte-blitz patch: images larger than an atlas page, each in a texture of its own.
    pub(crate) own_textures: &'a mut FxHashMap<u64, OwnTexture>,
    /// svelte-blitz patch: the atlas page size, `(width, height)`.
    pub(crate) atlas_page: (u16, u16),
}

/// svelte-blitz patch: an image too large for an atlas page. vello_gpu's
/// `upload_image` panics on those (`TextureTooLarge`), so they are drawn as
/// external textures instead, which also lets the atlas pages stay small.
pub struct OwnTexture {
    pub(crate) id: TextureId,
    pub(crate) view: TextureView,
    region: RectU16,
}

/// Ids of [`OwnTexture`]s: apart from anyrender's `ResourceId`s, which count up from 0.
const OWN_TEXTURE_ID: u64 = 1 << 63;

impl<'a> ImageManager<'a> {
    pub fn new(
        renderer: &'a mut Renderer,
        resources: &'a mut Resources,
        device: &'a Device,
        queue: &'a Queue,
        encoder: &'a mut CommandEncoder,
        cache: &'a mut FxHashMap<u64, ImageId>,
        own_textures: &'a mut FxHashMap<u64, OwnTexture>,
        atlas_page: (u16, u16),
    ) -> Self {
        Self {
            renderer,
            resources,
            device,
            queue,
            encoder,
            cache,
            own_textures,
            atlas_page,
        }
    }

    /// svelte-blitz patch: where an image is drawn from, uploading it on first use.
    /// `None` for an image larger than the GPU can hold, which is not drawn.
    pub(crate) fn image_source(&mut self, image: &ImageData) -> Option<ImageSource> {
        let peniko_id = image.data.id();
        if let Some(own) = self.own_textures.get(&peniko_id) {
            return Some(ImageSource::external_texture(own.id, own.region, true));
        }
        let fits = image.width <= u32::from(self.atlas_page.0)
            && image.height <= u32::from(self.atlas_page.1);
        if fits {
            return Some(ImageSource::OpaqueId {
                id: self.upload_image(image),
                // TODO: optimize opaque case
                may_have_transparency: true,
            });
        }
        let limit = self.device.limits().max_texture_dimension_2d;
        if image.width > limit || image.height > limit || image.width == 0 || image.height == 0 {
            return None;
        }
        let ImageSource::Pixmap(pixmap) = ImageSource::from_peniko_image_data(image) else {
            unreachable!(); // ImageSource::from_peniko_image_data always return a Pixmap
        };
        let size = wgpu::Extent3d {
            width: u32::from(pixmap.width()),
            height: u32::from(pixmap.height()),
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Image"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // As vello_gpu's atlas pages: premultiplied RGBA8.
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixmap.data_as_u8_slice(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * size.width),
                rows_per_image: None,
            },
            size,
        );
        let own = OwnTexture {
            id: TextureId(OWN_TEXTURE_ID | self.own_textures.len() as u64),
            view: texture.create_view(&TextureViewDescriptor::default()),
            region: RectU16 {
                x0: 0,
                y0: 0,
                x1: pixmap.width(),
                y1: pixmap.height(),
            },
        };
        let source = ImageSource::external_texture(own.id, own.region, true);
        self.own_textures.insert(peniko_id, own);
        Some(source)
    }

    pub(crate) fn upload_image(&mut self, image: &ImageData) -> ImageId {
        let peniko_id = image.data.id();

        // Try to get ImageId from cache first
        if let Some(atlas_id) = self.cache.get(&peniko_id) {
            return *atlas_id;
        };

        // Convert ImageData to Pixmap
        let ImageSource::Pixmap(pixmap) = ImageSource::from_peniko_image_data(image) else {
            unreachable!(); // ImageSource::from_peniko_image_data always return a Pixmap
        };

        // Upload Pixamp
        let atlas_id = self.renderer.upload_image(
            self.resources,
            self.device,
            self.queue,
            self.encoder,
            &pixmap,
        );

        // Store ImageId in cache
        self.cache.insert(peniko_id, atlas_id);

        // Return ImageId
        atlas_id
    }
}

pub(crate) enum LayerKind {
    Layer,
    Clip,
}

pub struct VelloHybridScenePainter<'s> {
    /// svelte-blitz patch: when set, drawing commands are recorded here instead of
    /// drawn, to be compared with the last frame's (`damage.rs`).
    pub(crate) recorder: Option<&'s mut anyrender::Scene>,
    pub(crate) scene: &'s mut vello_gpu::Scene,
    pub(crate) layer_stack: Vec<LayerKind>,
    pub(crate) image_manager: ImageManager<'s>,
    pub(crate) texture_bindings: &'s mut FxHashMap<ResourceId, TextureView>,
    pub(crate) device_handle: &'s DeviceHandle,
}

impl VelloHybridScenePainter<'_> {
    pub fn new<'s>(
        scene: &'s mut vello_gpu::Scene,
        image_manager: ImageManager<'s>,
        texture_bindings: &'s mut FxHashMap<ResourceId, TextureView>,
        device_handle: &'s DeviceHandle,
    ) -> VelloHybridScenePainter<'s> {
        VelloHybridScenePainter {
            recorder: None,
            scene,
            layer_stack: Vec::with_capacity(16),
            image_manager,
            texture_bindings,
            device_handle,
        }
    }

    fn convert_paint(&mut self, paint: PaintRef<'_>) -> PaintType {
        anyrender_paint_to_vello_hybrid_paint(paint, &mut self.image_manager, self.texture_bindings)
    }
}

impl RenderContext for VelloHybridScenePainter<'_> {
    fn try_register_custom_resource(
        &mut self,
        resource: Box<dyn std::any::Any>,
    ) -> Result<ResourceId, anyrender::RegisterResourceError> {
        // Try to downcast as Texture
        match resource.downcast::<Texture>() {
            Ok(texture) => {
                let id = ResourceId::new();
                let texture_view = texture.create_view(&TextureViewDescriptor::default());
                self.texture_bindings.insert(id, texture_view);
                Ok(id)
            }
            Err(resource) => {
                // Else try to downcast as TextureView
                if let Ok(texture_view) = resource.downcast::<TextureView>() {
                    let id = ResourceId::new();
                    self.texture_bindings.insert(id, *texture_view);
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
        self.texture_bindings.remove(&resource_id);
    }

    fn renderer_specific_context(&self) -> Option<Box<dyn std::any::Any>> {
        Some(Box::new(self.device_handle.clone()) as _)
    }
}

impl PaintScene for VelloHybridScenePainter<'_> {
    fn reset(&mut self) {
        if let Some(r) = self.recorder.as_deref_mut() {
            return r.reset();
        }
        self.scene.reset();
    }

    fn push_layer(
        &mut self,
        fill: Fill,
        blend: impl Into<BlendMode>,
        alpha: f32,
        transform: Affine,
        clip: &impl Shape,
        filter: Option<Arc<Filter>>,
        _backdrop_filter: Option<Arc<Filter>>,
    ) {
        if let Some(r) = self.recorder.as_deref_mut() {
            return r.push_layer(fill, blend, alpha, transform, clip, filter, _backdrop_filter);
        }
        let filter = filter.and_then(crate::filters::convert_filter);
        let blend: BlendMode = blend.into();
        self.scene.set_transform(transform);
        self.scene.set_fill_rule(fill);
        // svelte-blitz patch: a layer that only groups (opaque, normal blending,
        // no filter this renderer draws: backdrop filters are not drawn at all)
        // composites exactly as its contents drawn directly, so it is drawn as a
        // clip, without a layer texture. A dialog overlay is such a layer only
        // because it has a backdrop-filter, and its texture is window-sized.
        if alpha == 1.0 && blend == BlendMode::default() && filter.is_none() {
            self.layer_stack.push(LayerKind::Clip);
            self.scene
                .push_clip_path(&clip.into_path(DEFAULT_TOLERANCE));
            return;
        }
        self.layer_stack.push(LayerKind::Layer);
        self.scene
            .push_clip_path(&clip.into_path(DEFAULT_TOLERANCE));
        self.scene
            .push_layer(None, Some(blend), Some(alpha), None, filter);
    }

    fn push_clip_layer(&mut self, fill: Fill, transform: Affine, clip: &impl Shape) {
        if let Some(r) = self.recorder.as_deref_mut() {
            return r.push_clip_layer(fill, transform, clip);
        }
        self.scene.set_transform(transform);
        self.scene.set_fill_rule(fill);
        self.layer_stack.push(LayerKind::Clip);
        self.scene
            .push_clip_path(&clip.into_path(DEFAULT_TOLERANCE));
    }

    fn pop_layer(&mut self) {
        if let Some(r) = self.recorder.as_deref_mut() {
            return r.pop_layer();
        }
        if let Some(kind) = self.layer_stack.pop() {
            match kind {
                LayerKind::Layer => {
                    self.scene.pop_layer();
                    self.scene.pop_clip();
                }
                LayerKind::Clip => self.scene.pop_clip(),
            }
        }
    }

    fn stroke<'a>(
        &mut self,
        style: &Stroke,
        transform: Affine,
        paint: impl Into<PaintRef<'a>>,
        brush_transform: Option<Affine>,
        shape: &impl Shape,
    ) {
        if let Some(r) = self.recorder.as_deref_mut() {
            return r.stroke(style, transform, paint, brush_transform, shape);
        }
        self.scene.set_transform(transform);
        self.scene.set_stroke(style.clone());
        let paint = self.convert_paint(paint.into());
        self.scene.set_paint(paint);
        self.scene
            .set_paint_transform(brush_transform.unwrap_or(Affine::IDENTITY));
        self.scene.stroke_path(&shape.into_path(DEFAULT_TOLERANCE));
    }

    fn fill<'a>(
        &mut self,
        style: Fill,
        transform: Affine,
        paint: impl Into<PaintRef<'a>>,
        brush_transform: Option<Affine>,
        shape: &impl Shape,
    ) {
        if let Some(r) = self.recorder.as_deref_mut() {
            return r.fill(style, transform, paint, brush_transform, shape);
        }
        self.scene.set_transform(transform);
        self.scene.set_fill_rule(style);
        let paint = self.convert_paint(paint.into());
        self.scene.set_paint(paint);
        self.scene
            .set_paint_transform(brush_transform.unwrap_or(Affine::IDENTITY));
        self.scene.fill_path(&shape.into_path(DEFAULT_TOLERANCE));
    }

    fn draw_glyphs<'a, 's: 'a>(
        &'a mut self,
        font: &'a FontData,
        font_size: f32,
        hint: bool,
        normalized_coords: &'a [NormalizedCoord],
        embolden: kurbo::Vec2,
        style: impl Into<StyleRef<'a>>,
        paint: impl Into<PaintRef<'a>>,
        _brush_alpha: f32,
        transform: Affine,
        glyph_transform: Option<Affine>,
        glyphs: impl Iterator<Item = anyrender::Glyph> + Clone,
    ) {
        // Borrowed inside the branch: a borrow for `'a` taken in the condition
        // would outlive the rest of the function.
        if self.recorder.is_some() {
            let r = self.recorder.as_deref_mut().expect("checked above");
            return r.draw_glyphs(
                font,
                font_size,
                hint,
                normalized_coords,
                embolden,
                style,
                paint,
                _brush_alpha,
                transform,
                glyph_transform,
                glyphs,
            );
        }
        let paint = self.convert_paint(paint.into());
        self.scene.set_paint(paint);
        self.scene.reset_paint_transform();
        self.scene.set_transform(transform);

        let style: StyleRef<'a> = style.into();
        match style {
            StyleRef::Fill(fill) => {
                self.scene.set_fill_rule(fill);
                let _ = self
                    .scene
                    .glyph_run(self.image_manager.resources, font)
                    .font_size(font_size)
                    .hint(hint)
                    .normalized_coords(normalized_coords)
                    .font_embolden(FontEmbolden::new(Diagonal2::new(embolden.x, embolden.y)))
                    .glyph_transform(glyph_transform.unwrap_or_default())
                    // svelte-blitz patch: upstream leaves the glyph atlas off, so
                    // every glyph is rasterised again for every frame.
                    .atlas_cache(crate::glyph_cache())
                    .fill_glyphs(glyphs.map(|g| glifo::Glyph {
                        id: g.id,
                        x: g.x,
                        y: g.y,
                    }));
            }
            StyleRef::Stroke(stroke) => {
                self.scene.set_stroke(stroke.clone());
                let _ = self
                    .scene
                    .glyph_run(self.image_manager.resources, font)
                    .font_size(font_size)
                    .hint(hint)
                    .normalized_coords(normalized_coords)
                    .glyph_transform(glyph_transform.unwrap_or_default())
                    .atlas_cache(crate::glyph_cache()) // svelte-blitz patch
                    .stroke_glyphs(glyphs.map(|g| glifo::Glyph {
                        id: g.id,
                        x: g.x,
                        y: g.y,
                    }));
            }
        }
    }
    fn draw_box_shadow(
        &mut self,
        transform: Affine,
        rect: Rect,
        color: Color,
        radius: f64,
        std_dev: f64,
    ) {
        if let Some(r) = self.recorder.as_deref_mut() {
            return r.draw_box_shadow(transform, rect, color, radius, std_dev);
        }
        self.scene.set_transform(transform);
        self.scene.set_paint(PaintType::Solid(color));
        self.scene.reset_paint_transform();
        self.scene
            .fill_blurred_rounded_rect(&rect, radius as f32, std_dev as f32, false);
    }
}
