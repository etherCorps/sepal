use std::sync::Arc;

use anyrender::filters::{EdgeMode, Filter, FilterEffect};
use vello_common::filter_effects::FilterPrimitive;

/// sepal patch: whether this renderer draws CSS backdrop filters. It does
/// not (`VelloHybridScenePainter::push_layer` ignores them), so a layer whose only
/// filter is a backdrop filter draws exactly like one without. Damage tracking
/// (`damage::is_filter_layer`) and drawing such a layer as a clip
/// (`damage::grouping_only`) rely on it; both have to change when backdrop
/// filters are drawn.
pub(crate) const BACKDROP_FILTERS_DRAWN: bool = false;

pub(crate) fn convert_filter(filter: Arc<Filter>) -> Option<vello_common::filter_effects::Filter> {
    let nodes = filter.nodes();
    if nodes.is_empty() {
        return None;
    }

    // Vello Hybrid only supports single-node filters at the moment
    let node = &filter.nodes()[0];
    let primitive = convert_filter_effect(&node.effect)?;
    Some(vello_common::filter_effects::Filter::from_primitive(
        primitive,
    ))
}

pub(crate) fn convert_filter_effect(effect: &FilterEffect) -> Option<FilterPrimitive> {
    Some(match effect {
        FilterEffect::Flood(color) => FilterPrimitive::Flood { color: *color },
        FilterEffect::GaussianBlur(blur) => FilterPrimitive::GaussianBlur {
            std_deviation: blur.std_deviation,
            edge_mode: convert_edge_mode(blur.edge_mode),
        },
        FilterEffect::DropShadow(shadow) => FilterPrimitive::DropShadow {
            dx: shadow.dx,
            dy: shadow.dy,
            std_deviation: shadow.std_deviation,
            color: shadow.color,
            edge_mode: convert_edge_mode(shadow.edge_mode),
        },
        FilterEffect::Offset(offset) => FilterPrimitive::Offset {
            dx: offset.x as f32,
            dy: offset.y as f32,
        },
        FilterEffect::ColorMatrix(_matrix) => return None, //FilterPrimitive::ColorMatrix { matrix: matrix.0 },
        FilterEffect::Blend(_mode) => return None,         //FilterPrimitive::Blend { mode: *mode },
        FilterEffect::ComponentTransfer(_component_transfer_filter) => return None,
        FilterEffect::Composite(_composite_operator) => return None,
        FilterEffect::Morphology(_morphology_filter) => return None,
        FilterEffect::ConvolveMatrix(_convolution_kernel) => return None,
        FilterEffect::Turbulence(_turbulence_filter) => return None,
        FilterEffect::DisplacementMap(_displacement_map_filter) => return None,
        FilterEffect::Image(_external_image_source) => return None,
        FilterEffect::Tile => return None,
        FilterEffect::DiffuseLighting(_diffuse_lighting_filter) => return None,
        FilterEffect::SpecularLighting(_specular_lighting_filter) => return None,
    })
}

fn convert_edge_mode(edge_mode: EdgeMode) -> vello_common::filter_effects::EdgeMode {
    match edge_mode {
        EdgeMode::Duplicate => vello_common::filter_effects::EdgeMode::Duplicate,
        EdgeMode::Wrap => vello_common::filter_effects::EdgeMode::Wrap,
        EdgeMode::Mirror => vello_common::filter_effects::EdgeMode::Mirror,
        EdgeMode::None => vello_common::filter_effects::EdgeMode::None,
    }
}
