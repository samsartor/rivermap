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
    let history: vec4<f32> = textureSample(history_tex, tex_sampler, tex_coords);
    let fill: vec4<f32> = textureSample(fill_tex, tex_sampler, tex_coords);
    let border: vec4<f32> = textureSample(border_tex, tex_sampler, tex_coords);
    return FragmentOutput(alpha_over(border, alpha_over(fill, history)));
}

fn alpha_over(fg: vec4<f32>, bg: vec4<f32>) -> vec4<f32> {
    let out_a = fg.a + bg.a * (1.0 - fg.a);
    let out_rgb = (fg.rgb * fg.a + bg.rgb * bg.a * (1.0 - fg.a)) / out_a;
    return vec4<f32>(out_rgb, out_a);
}
