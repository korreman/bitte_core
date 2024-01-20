// The only thing this shader does is to propagate a position and sample a texture.

@group(0) @binding(0)
var texture: texture_2d<f32>;

@group(0) @binding(1)
var sampler_diffuse: sampler;

struct UpscaleInter {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn upscale_v(
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
) -> UpscaleInter {
    return UpscaleInter (
        vec4<f32>(position, 0.0, 1.0),
        tex_coords,
    );
}

@fragment
fn upscale_f(
    in: UpscaleInter,
) -> @location(0) vec4<f32> {
    return textureSample(texture, sampler_diffuse, in.tex_coords);
}
