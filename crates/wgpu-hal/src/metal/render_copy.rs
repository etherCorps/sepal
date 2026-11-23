//! svelte-blitz patch: buffer clears and copies done in render passes.
//!
//! On Apple GPUs the driver commits about 80 MB of memory the first time a
//! process runs any blit or compute work, on top of the ~120 MB its first render
//! pass costs, and keeps it for as long as frames are drawn. wgpu uses blit
//! encoders for every upload (`Queue::write_buffer`/`write_texture` copy from a
//! staging buffer) and for zero-initialising resources, so a window drawn with
//! wgpu paid it although it never needs a blit. The same work done in render
//! encoders stays on the GPU timeline, in order with everything else, and does
//! not trigger it: buffer words are written by a vertex function with
//! rasterization off, texture texels by a fragment function that reads the
//! source buffer. Formats without such a function, 3D textures and the rare
//! texture-to-texture and texture-to-buffer copies still use blits.
//!
//! `SVELTE_BLITZ_GPU_COPIES=blit` turns this off.

use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_foundation::NSString;
use objc2_metal::{
    MTLBuffer, MTLCommandBuffer, MTLCommandEncoder, MTLCompileOptions, MTLDevice, MTLGPUFamily,
    MTLLibrary, MTLLoadAction, MTLPixelFormat, MTLPrimitiveType, MTLRenderCommandEncoder,
    MTLRenderPassDescriptor, MTLRenderPipelineDescriptor, MTLRenderPipelineState, MTLScissorRect,
    MTLStoreAction, MTLTexture, MTLViewport,
};

const SOURCE: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Words { uint src; uint dst; };
vertex void copy_words(uint i [[vertex_id]], const device uint* src [[buffer(0)]],
                       device uint* dst [[buffer(1)]], constant Words& w [[buffer(2)]]) {
    dst[w.dst + i] = src[w.src + i];
}
vertex void zero_words(uint i [[vertex_id]], device uint* dst [[buffer(1)]],
                       constant Words& w [[buffer(2)]]) {
    dst[w.dst + i] = 0;
}

struct Cover { float4 position [[position]]; };
vertex Cover cover(uint i [[vertex_id]]) {
    float2 uv = float2((i << 1) & 2, i & 2);
    return { float4(uv * 2 - 1, 0, 1) };
}
struct Texels { uint src; uint row_words; uint2 origin; };
fragment uint4 to_rgba32uint(Cover c [[stage_in]], const device uint* src [[buffer(0)]],
                             constant Texels& t [[buffer(1)]]) {
    uint2 p = uint2(c.position.xy) - t.origin;
    uint w = t.src + p.y * t.row_words + p.x * 4;
    return uint4(src[w], src[w + 1], src[w + 2], src[w + 3]);
}
fragment float4 to_rgba8unorm(Cover c [[stage_in]], const device uint* src [[buffer(0)]],
                              constant Texels& t [[buffer(1)]]) {
    uint2 p = uint2(c.position.xy) - t.origin;
    return unpack_unorm4x8_to_float(src[t.src + p.y * t.row_words + p.x]);
}
"#;

#[repr(C)]
struct Words {
    src: u32,
    dst: u32,
}

#[repr(C)]
struct Texels {
    src: u32,
    row_words: u32,
    origin: [u32; 2],
}

type Pipeline = Retained<ProtocolObject<dyn MTLRenderPipelineState>>;

#[derive(Debug)]
pub(super) struct RenderCopy {
    copy_words: Pipeline,
    zero_words: Pipeline,
    to_rgba32uint: Pipeline,
    to_rgba8unorm: Pipeline,
}

// Pipeline states are immutable and thread-safe in Metal.
unsafe impl Send for RenderCopy {}
unsafe impl Sync for RenderCopy {}

/// What `copy_buffer_to_texture` hands over for one region.
pub(super) struct TexelCopy<'a> {
    pub src: &'a ProtocolObject<dyn MTLBuffer>,
    pub src_offset: u64,
    pub bytes_per_row: u64,
    pub dst: &'a ProtocolObject<dyn MTLTexture>,
    pub level: usize,
    pub slice: usize,
    pub origin: [u32; 2],
    pub size: [u32; 2],
}

impl RenderCopy {
    /// Whether this is in use on `device`: Apple GPUs only, where the driver
    /// behaves as described above.
    pub(super) fn wanted(device: &ProtocolObject<dyn MTLDevice>) -> bool {
        std::env::var("SVELTE_BLITZ_GPU_COPIES").as_deref() != Ok("blit")
            && device.supportsFamily(MTLGPUFamily::Apple7)
    }

    pub(super) fn new(device: &ProtocolObject<dyn MTLDevice>) -> Option<Self> {
        let library = device
            .newLibraryWithSource_options_error(
                &NSString::from_str(SOURCE),
                Some(&MTLCompileOptions::new()),
            )
            .map_err(|e| log::warn!("render copies off, MSL: {e:?}"))
            .ok()?;
        let function = |name: &str| library.newFunctionWithName(&NSString::from_str(name));
        let pipeline =
            |vertex: &str, fragment: Option<(&str, MTLPixelFormat)>| -> Option<Pipeline> {
                let descriptor = MTLRenderPipelineDescriptor::new();
                descriptor.setVertexFunction(Some(&*function(vertex)?));
                match fragment {
                    Some((name, format)) => {
                        descriptor.setFragmentFunction(Some(&*function(name)?));
                        unsafe { descriptor.colorAttachments().objectAtIndexedSubscript(0) }
                            .setPixelFormat(format);
                    }
                    None => descriptor.setRasterizationEnabled(false),
                }
                device
                    .newRenderPipelineStateWithDescriptor_error(&descriptor)
                    .map_err(|e| log::warn!("render copies off, pipeline {vertex}: {e:?}"))
                    .ok()
            };
        Some(Self {
            copy_words: pipeline("copy_words", None)?,
            zero_words: pipeline("zero_words", None)?,
            to_rgba32uint: pipeline("cover", Some(("to_rgba32uint", MTLPixelFormat::RGBA32Uint)))?,
            to_rgba8unorm: pipeline("cover", Some(("to_rgba8unorm", MTLPixelFormat::RGBA8Unorm)))?,
        })
    }

    /// Texture formats `copy_to_texture` can write.
    pub(super) fn supports(format: wgt::TextureFormat) -> bool {
        matches!(
            format,
            wgt::TextureFormat::Rgba32Uint | wgt::TextureFormat::Rgba8Unorm
        )
    }

    /// An encoder with nothing attached: vertex functions write buffers.
    fn buffer_pass(
        cmd_buf: &ProtocolObject<dyn MTLCommandBuffer>,
    ) -> Retained<ProtocolObject<dyn MTLRenderCommandEncoder>> {
        let descriptor = MTLRenderPassDescriptor::new();
        descriptor.setRenderTargetWidth(1);
        descriptor.setRenderTargetHeight(1);
        descriptor.setDefaultRasterSampleCount(1);
        cmd_buf
            .renderCommandEncoderWithDescriptor(&descriptor)
            .expect("render encoder for a buffer copy")
    }

    fn words(
        encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>,
        src: Option<&ProtocolObject<dyn MTLBuffer>>,
        dst: &ProtocolObject<dyn MTLBuffer>,
        src_offset: u64,
        dst_offset: u64,
        size: u64,
    ) {
        // Copy and clear offsets and sizes are multiples of 4 (COPY_BUFFER_ALIGNMENT).
        let words = Words {
            src: (src_offset / 4) as u32,
            dst: (dst_offset / 4) as u32,
        };
        unsafe {
            if let Some(src) = src {
                encoder.setVertexBuffer_offset_atIndex(Some(src), 0, 0);
            }
            encoder.setVertexBuffer_offset_atIndex(Some(dst), 0, 1);
            encoder.setVertexBytes_length_atIndex(
                core::ptr::NonNull::from(&words).cast(),
                size_of::<Words>(),
                2,
            );
            encoder.drawPrimitives_vertexStart_vertexCount(
                MTLPrimitiveType::Point,
                0,
                (size / 4) as usize,
            );
        }
    }

    pub(super) fn clear_buffer(
        &self,
        cmd_buf: &ProtocolObject<dyn MTLCommandBuffer>,
        dst: &ProtocolObject<dyn MTLBuffer>,
        range: core::ops::Range<u64>,
    ) {
        if range.is_empty() {
            return;
        }
        let encoder = Self::buffer_pass(cmd_buf);
        encoder.setRenderPipelineState(&self.zero_words);
        Self::words(&encoder, None, dst, 0, range.start, range.end - range.start);
        encoder.endEncoding();
    }

    pub(super) fn copy_buffer(
        &self,
        cmd_buf: &ProtocolObject<dyn MTLCommandBuffer>,
        src: &ProtocolObject<dyn MTLBuffer>,
        dst: &ProtocolObject<dyn MTLBuffer>,
        regions: impl Iterator<Item = crate::BufferCopy>,
    ) {
        let encoder = Self::buffer_pass(cmd_buf);
        encoder.setRenderPipelineState(&self.copy_words);
        for copy in regions {
            Self::words(
                &encoder,
                Some(src),
                dst,
                copy.src_offset,
                copy.dst_offset,
                copy.size.get(),
            );
        }
        encoder.endEncoding();
    }

    /// One region into a 2D slice of a texture `supports` accepts. The source
    /// offset and row pitch are multiples of the texel size.
    pub(super) fn copy_to_texture(
        &self,
        cmd_buf: &ProtocolObject<dyn MTLCommandBuffer>,
        format: wgt::TextureFormat,
        copy: TexelCopy<'_>,
    ) {
        if copy.size[0] == 0 || copy.size[1] == 0 {
            return;
        }
        let pipeline = match format {
            wgt::TextureFormat::Rgba32Uint => &self.to_rgba32uint,
            _ => &self.to_rgba8unorm,
        };
        let descriptor = MTLRenderPassDescriptor::new();
        let attachment = unsafe { descriptor.colorAttachments().objectAtIndexedSubscript(0) };
        attachment.setTexture(Some(copy.dst));
        attachment.setLevel(copy.level);
        attachment.setSlice(copy.slice);
        attachment.setLoadAction(MTLLoadAction::Load);
        attachment.setStoreAction(MTLStoreAction::Store);
        let encoder = cmd_buf
            .renderCommandEncoderWithDescriptor(&descriptor)
            .expect("render encoder for a texture copy");
        let width = (copy.dst.width() >> copy.level).max(1);
        let height = (copy.dst.height() >> copy.level).max(1);
        encoder.setRenderPipelineState(pipeline);
        // The viewport covers the whole level, so a fragment's position is its
        // texel; the scissor keeps the draw to the region.
        encoder.setViewport(MTLViewport {
            originX: 0.0,
            originY: 0.0,
            width: width as f64,
            height: height as f64,
            znear: 0.0,
            zfar: 1.0,
        });
        encoder.setScissorRect(MTLScissorRect {
            x: copy.origin[0] as usize,
            y: copy.origin[1] as usize,
            width: copy.size[0] as usize,
            height: copy.size[1] as usize,
        });
        let texels = Texels {
            src: (copy.src_offset / 4) as u32,
            row_words: (copy.bytes_per_row / 4) as u32,
            origin: copy.origin,
        };
        unsafe {
            encoder.setFragmentBuffer_offset_atIndex(Some(copy.src), 0, 0);
            encoder.setFragmentBytes_length_atIndex(
                core::ptr::NonNull::from(&texels).cast(),
                size_of::<Texels>(),
                1,
            );
            encoder.drawPrimitives_vertexStart_vertexCount(MTLPrimitiveType::Triangle, 0, 3);
        }
        encoder.endEncoding();
    }
}
