// Pixel dimensions of target canvas
struct CanvasInfo {
    @align(16) dimensions: vec2<u32>,
};

@group(0) @binding(0)
var<uniform> canvas_info: CanvasInfo;

@group(1) @binding(0)
var sprite_sheet: texture_2d<f32>;

// Utility functions

// Convert a pixel coordinate to a normalized device coordinate.
// The result will point to the center of the corresponding pixel on the canvas.
// We're currently assuming that (-1, -1) refers to the bottom left corner of the bottom left pixel.
// Not to the center of the bottom left pixel.
fn pixel_to_ndc(coord: vec2<f32>) -> vec4<f32> {
    let result = (0.5 + coord) / vec2<f32>(canvas_info.dimensions) * 2.0 - 1.0;
    return vec4<f32>(result, 0.0, 1.0);
}

// A super-simple shader for use in drawing primitives.
struct PrimitiveInter {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex fn primitive_v(
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
) -> PrimitiveInter {
    let ndc = pixel_to_ndc(vec2<f32>(position));
    return PrimitiveInter(ndc, color);
}

@fragment fn primitive_f(in: PrimitiveInter) -> @location(0) vec4<f32> { return in.color; }
/// Shader for drawing instanced rectangles
struct RectInter {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex fn rect_v(
    // Quad
    @location(0) position: vec2<f32>,
    // Instance
    @location(1) offset: vec2<i32>,
    @location(2) dimensions: vec2<u32>,
    @location(3) color: vec4<f32>,
) -> RectInter {
    // Line rasterization isn't defined in the spec yet.
    // WebGL and Vulkan backends seem to draw a rotated rectangle
    // from the starting point to finishing point with width 1px,
    // then include every pixel whose center is inside this rectangle.
    // We must stretch the lines slightly to ensure that corners are included in these rectangles.
    // This shouldn't be a problem if the Bresenham/diamond approach is later adopted.
    let stretch = 0.002;
    let pos = vec2<f32>(offset) + (vec2<f32>(dimensions - 1u) + stretch * 2.0) * position - stretch;
    let ndc = pixel_to_ndc(pos);
    return RectInter(ndc, color);
}

@fragment fn rect_f(in: RectInter) -> @location(0) vec4<f32> { return in.color; }

/// Sprite renderer
struct SpriteInter {
    // Resulting coordinate on canvas
    @builtin(position) position: vec4<f32>,
    // Texture lookup coordinate
    @location(0) texture_coord: vec2<f32>,
    // Sprite width, used for converting texture coordinate to 1D
    @location(1) width: u32,
    // Texture address
    @location(2) address: u32,
};

@vertex fn sprite_v(
    @location(0) position: vec2<f32>,
    // Desired position of the sprite on canvas
    @location(1) sprite_position: vec2<i32>,
    // Dimensions of sprite
    @location(2) dimensions: vec2<u32>,
    // Address of sprite in 1D sprite sheet
    @location(3) address: u32,
) -> SpriteInter {
    let pos = vec2<f32>(sprite_position) + vec2<f32>(dimensions) * position - 0.5;
    let ndc = pixel_to_ndc(pos);

    // Pixel coordinate inside sprite
    // TODO: Figure out if texture coordinates are stretched too far.
    let texture_coord: vec2<f32> =
        vec2<f32>(position.x, 1.0 - position.y) * vec2<f32>(dimensions - 1u);

    return SpriteInter (
        ndc,
        texture_coord,
        dimensions.x,
        address,
    );
}

@fragment fn sprite_f(
    in: SpriteInter,
) -> @location(0) vec4<f32> {
    // NOTE: The sprite_sheet is NOT actually a 2D sheet,
    // but rather a 1D array spread over two dimensions due to texture size limitations.
    // See the `sprite` module for details.

    // Compute the memory address from x and y coordinates
    let offset =
        u32(round(in.texture_coord.x)) +
        u32(round(in.texture_coord.y)) * in.width;
    let texture_address: u32 = in.address + offset;

    // Convert the address to a 2D coordinate
    // TODO: Remove cast once https://github.com/gfx-rs/naga/issues/2010 is resolved
    let sheet_width: u32 = u32(textureDimensions(sprite_sheet).x);
    let sheet_coord = vec2<u32>(texture_address % sheet_width, texture_address / sheet_width);

    // TODO: Remove cast to i32 once https://github.com/gfx-rs/naga/issues/1997 is resolved
    let result: vec4<f32> = textureLoad(sprite_sheet, vec2<i32>(sheet_coord), 0);

    return result;
}

// Circle renderer
struct CircleInter {
    @builtin(position) position: vec4<f32>,
    @location(0) circle_coord: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) pixel_size: f32,
}

@vertex fn circle_v(
    // Quad
    @location(0) position: vec2<f32>,
    // Instance
    @location(1) offset: vec2<i32>,
    @location(2) diameter: u32,
    @location(3) color: vec4<f32>,
) -> CircleInter {
    let pos = vec2<f32>(offset) + f32(diameter) * position - 0.5;
    let ndc = pixel_to_ndc(pos);

    let circle_coord: vec2<f32> = 2.0 * position - 1.0;

    return CircleInter (ndc, circle_coord, color, 2.0 / f32(diameter));
}

@fragment fn circle_f(
    in: CircleInter,
) -> @location(0) vec4<f32> {
    let d = length(in.circle_coord);
    if 1.0 - in.pixel_size <= d && d <= 1.0 {
        return in.color;
    } else {
        return vec4<f32>(0.0);
    }
}
