// PNG boundary data is straight-alpha sRGB. Every texture is linear premultiplied.
@group(0) @binding(0) var<storage, read> png_pixels: array<u32>;
@group(0) @binding(1) var foreground_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var background_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(3) var<uniform> background_rgba: vec4<u32>;

fn linear_channel(v: f32) -> f32 {
    if v <= 0.04045 { return v / 12.92; }
    return pow((v + 0.055) / 1.055, 2.4);
}
fn ingest_rgba(packed: u32) -> vec4<f32> {
    let c = vec4<f32>(f32(packed & 255u), f32((packed >> 8u) & 255u),
        f32((packed >> 16u) & 255u), f32(packed >> 24u)) / 255.0;
    return vec4<f32>(vec3<f32>(linear_channel(c.r), linear_channel(c.g),
        linear_channel(c.b)) * c.a, c.a);
}
@compute @workgroup_size(8, 8)
fn ingest(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(foreground_out);
    if any(id.xy >= size) { return; }
    textureStore(foreground_out, id.xy, ingest_rgba(png_pixels[id.y * size.x + id.x]));
    textureStore(background_out, id.xy, ingest_rgba(background_rgba.x));
}

@group(0) @binding(4) var foreground_in: texture_2d<f32>;
@group(0) @binding(5) var background_in: texture_2d<f32>;
@group(0) @binding(6) var composite_out: texture_storage_2d<rgba16float, write>;
@compute @workgroup_size(8, 8)
fn composite(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= textureDimensions(composite_out)) { return; }
    let fg = textureLoad(foreground_in, id.xy, 0);
    let bg = textureLoad(background_in, id.xy, 0);
    textureStore(composite_out, id.xy, fg + bg * (1.0 - fg.a));
}

@group(0) @binding(7) var composite_in: texture_2d<f32>;
@group(0) @binding(8) var<storage, read_write> exported_png: array<u32>;
fn srgb_channel(v: f32) -> f32 {
    if v <= 0.0031308 { return 12.92 * v; }
    return 1.055 * pow(v, 1.0 / 2.4) - 0.055;
}
@compute @workgroup_size(8, 8)
fn egress(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(composite_in);
    if any(id.xy >= size) { return; }
    let p = textureLoad(composite_in, id.xy, 0);
    var rgb = vec3<f32>(0.0);
    if p.a > 0.0 { rgb = p.rgb / p.a; }
    let srgb = vec4<f32>(srgb_channel(rgb.r), srgb_channel(rgb.g), srgb_channel(rgb.b), p.a);
    let bytes = vec4<u32>(round(clamp(srgb, vec4<f32>(0.0), vec4<f32>(1.0)) * 255.0));
    exported_png[id.y * size.x + id.x] = bytes.r | (bytes.g << 8u) | (bytes.b << 16u) | (bytes.a << 24u);
}
