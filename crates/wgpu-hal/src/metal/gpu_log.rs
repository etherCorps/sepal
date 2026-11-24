//! svelte-blitz patch: `SVELTE_BLITZ_GPU_LOG=1` prints on stderr every command
//! buffer, pass and copy this backend encodes, and every submit and present:
//! what one frame asks of the GPU, without a Metal capture. Off, it costs one
//! cached check per call.

use alloc::{borrow::ToOwned, format, string::String};
use std::sync::OnceLock;

use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLLoadAction, MTLPixelFormat, MTLStoreAction, MTLTexture};

pub(super) fn on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("SVELTE_BLITZ_GPU_LOG").is_some_and(|v| v == "1"))
}

pub(super) fn line(what: std::fmt::Arguments<'_>) {
    std::eprintln!("[gpu] {what}");
}

/// `2400x1600 BGRA8Unorm`.
pub(super) fn texture(t: &ProtocolObject<dyn MTLTexture>) -> String {
    let format = match t.pixelFormat() {
        MTLPixelFormat::RGBA8Unorm => "RGBA8Unorm".to_owned(),
        MTLPixelFormat::BGRA8Unorm => "BGRA8Unorm".to_owned(),
        MTLPixelFormat::RGBA8Unorm_sRGB => "RGBA8Unorm_sRGB".to_owned(),
        MTLPixelFormat::BGRA8Unorm_sRGB => "BGRA8Unorm_sRGB".to_owned(),
        MTLPixelFormat::RGBA32Uint => "RGBA32Uint".to_owned(),
        MTLPixelFormat::R8Unorm => "R8Unorm".to_owned(),
        MTLPixelFormat::Depth32Float => "Depth32Float".to_owned(),
        MTLPixelFormat::Depth32Float_Stencil8 => "Depth32Float_Stencil8".to_owned(),
        other => format!("{other:?}"),
    };
    let layers = t.arrayLength();
    let layers = if layers > 1 { format!(" x{layers} layers") } else { String::new() };
    format!("{}x{} {format}{layers}", t.width(), t.height())
}

pub(super) fn load(action: MTLLoadAction) -> &'static str {
    match action {
        MTLLoadAction::Load => "load",
        MTLLoadAction::Clear => "clear",
        _ => "dontcare",
    }
}

pub(super) fn store(action: MTLStoreAction) -> &'static str {
    match action {
        MTLStoreAction::Store => "store",
        MTLStoreAction::DontCare => "discard",
        _ => "other",
    }
}
