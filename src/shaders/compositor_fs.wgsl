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
    let age = textureSample(history_tex, tex_sampler, tex_coords).r;
    let hex_center = pointy_hex_to_pixel(round(pixel_to_pointy_hex(tex_coords)));
    let dist = length(tex_coords - hex_center);
    if dist < (0.5 * size) || age > 0.99 {
        return vec4(0.0, 0.0, 0.0, 0.0);
    } else {
        return vec4(gradient(age), 1.0);
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

fn gamma_correct(c: f32) -> f32 {
    return select(12.92 * c, 1.055 * pow(c, 1.0 / 2.4) - 0.055, c > 0.0031308);
}

fn oklch_to_srgb(oklch: vec3<f32>) -> vec3<f32> {
    // Unpack input
    let L = oklch.x;
    let C = oklch.y;
    let h = radians(oklch.z);

    // 1. OKLCH -> OKLab
    let a = C * cos(h);
    let b = C * sin(h);

    // 2. OKLab -> linear sRGB
    let l_ = L + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = L - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = L - 0.0894841775 * a - 1.2914855480 * b;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    var rgb_linear = vec3<f32>(
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
    );

    // 3. linear sRGB -> non-linear sRGB
    return vec3<f32>(
        gamma_correct(rgb_linear.x),
        gamma_correct(rgb_linear.y),
        gamma_correct(rgb_linear.z)
    );
}

const num_grad_stops: u32 = 3u;

fn gradient(pos: f32) -> vec3<f32> {
    var grad_stops: array<vec3<f32>, num_grad_stops> = array(
        vec3(0.7, 0.1135, 48.18),
        vec3(0.7, 0.1135, 139.41),
        vec3(0.7, 0.1135, 239.82),
    );
    
    let pos_mul = clamp(pos, 0.0, 1.0) * f32(num_grad_stops);
    let t = fract(pos_mul);
    let i = u32(pos_mul);
    let j = min(i + 1u, num_grad_stops - 1u);
    let color = mix(grad_stops[i], grad_stops[j], t);
    return oklch_to_srgb(color);
}
