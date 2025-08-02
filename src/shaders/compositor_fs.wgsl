struct FragmentOutput {
    @location(0) f_color: vec4<f32>,
};

@group(0) @binding(0)
var history_tex: texture_2d<f32>;
@group(0) @binding(1)
var border_tex: texture_2d<f32>;
@group(0) @binding(2)
var fill_tex: texture_2d<f32>;
@group(0) @binding(3)
var tex_sampler: sampler;

@fragment
fn main(@location(0) tex_coords: vec2<f32>) -> FragmentOutput {
    let history: vec4<f32> = history_color(tex_coords);
    let fill: vec4<f32> = textureSample(fill_tex, tex_sampler, tex_coords);
    let border: vec4<f32> = textureSample(border_tex, tex_sampler, tex_coords);
    return FragmentOutput(alpha_over(border, alpha_over(fill, alpha_over(history, vec4(1.0, 1.0, 1.0, 1.0)))));
}

fn history_color(tex_coords: vec2<f32>) -> vec4<f32> {
    let hex_center = pointy_hex_to_pixel(round(pixel_to_pointy_hex(tex_coords)));
    let dist = length(tex_coords - hex_center);
    let sampled = textureSample(history_tex, tex_sampler, tex_coords);
    if dist < (0.5 * size) {
        return vec4(0.0, 0.0, 0.0, 0.0);
    } else {
        return sampled;
    }
}

const size: f32 = 0.01;

fn pixel_to_pointy_hex(point: vec2<f32>) -> vec2<f32> {
    // invert the scaling
    let x = point.x / size;
    let y = point.y / size;
    // cartesian to hex
    let q = (sqrt(3.0) / 3.0 * x - 1.0 / 3.0 * y);
    let r = (2.0 / 3.0 * y);
    return vec2(q, r);
}

fn pointy_hex_to_pixel(hex: vec2<f32>) -> vec2<f32> {
    // hex to cartesian
    var x = (sqrt(3.0) * hex.x + sqrt(3.0) / 2.0 * hex.y);
    var y = (3.0 / 2.0 * hex.y);
    // scale cartesian coordinates
    x = x * size;
    y = y * size;
    return vec2(x, y);
}

fn alpha_over(fg: vec4<f32>, bg: vec4<f32>) -> vec4<f32> {
    let out_a = fg.a + bg.a * (1.0 - fg.a);
    let out_rgb = (fg.rgb * fg.a + bg.rgb * bg.a * (1.0 - fg.a)) / out_a;
    return vec4<f32>(out_rgb, out_a);
}
