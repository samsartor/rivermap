struct FragmentOutput {
    @location(0) f_color: vec4<f32>,
};

@group(0) @binding(0)
var history_tex: texture_multisampled_2d<f32>;
@group(0) @binding(1)
var border_tex: texture_multisampled_2d<f32>;
@group(0) @binding(2)
var fill_tex: texture_multisampled_2d<f32>;

@fragment
fn main(
    @location(0) tex_coords: vec2<f32>,
    @builtin(sample_index) sample_index: u32,
) -> FragmentOutput {
    let tex_size: vec2<u32> = textureDimensions(history_tex);
    let tex_x: i32 = i32(f32(tex_size.x) * tex_coords.x);
    let tex_y: i32 = i32(f32(tex_size.y) * tex_coords.y);
    let itex_coords: vec2<i32> = vec2<i32>(tex_x, tex_y);

    var history: vec4<f32> = textureLoad(history_tex, itex_coords, i32(sample_index));
    history = history_color(tex_coords, history);
    let paper = paper(tex_coords);
    let fill: vec4<f32> = paper * textureLoad(fill_tex, itex_coords, i32(sample_index)).a;
    let border: vec4<f32> = textureLoad(border_tex, itex_coords, i32(sample_index));
    return FragmentOutput(alpha_over(border, alpha_over(fill, alpha_over(history, paper))));
}

fn paper(loc: vec2<f32>) -> vec4<f32> {
    let color = vec3(0.8386, 0.052, 84.51);
    let darkest = vec3(0.7920, 0.057, 85.00);
    let large = (keep_over(simplex2d(loc * 10.0), 0.5) - 0.5) * 0.5;
    let many = min(normal_range(simplex2d(loc * 100.0)), normal_range(simplex2d(loc * 100.0 + 100.0))) * 0.7;
    let darken = clamp(large + many, 0.0, 1.0);
    let folded = max(fold(loc.x, 4.0), fold(loc.y, 3.0));
    let shifted = mix(color, darkest, min(folded, 1.0) + darken);

    return vec4(oklch_to_lin(shifted), 1.0);
}

fn fold(loc: f32, num_folds: f32) -> f32 {
    let dist = abs(fract(loc * num_folds) - 0.5) * 2.0;
    let lines = pow(20000000.0, dist) / 20000000.0;
    return lines * 1.0;
}

fn keep_over(keep: f32, over: f32) -> f32 {
    if keep >= over {
        return keep;
    } else {
        return 0.0;
    }
}

fn normal_range(in: f32) -> f32 {
    return (in + 1.0) / 2.0;
}

// Next two copied from https://www.shadertoy.com/view/Msf3WH
fn hash(p: vec2<f32>) -> vec2<f32> {
    let p2 = vec2(dot(p, vec2(127.1, 311.7)), dot(p, vec2(269.5, 183.3)));
    return -1.0 + 2.0 * fract(sin(p2) * 43758.5453123);
}

fn simplex2d(p: vec2<f32>) -> f32 {
    let K1 = 0.366025404; // (sqrt(3)-1)/2;
    let K2 = 0.211324865; // (3-sqrt(3))/6;
    let i = floor(p + (p.x + p.y) * K1);
    let a = p - i + (i.x + i.y) * K2;
    let o = step(a.yx, a.xy);
    let b = a - o + K2;
    let c = a - 1.0 + 2.0 * K2;
    let h = max(0.5 - vec3(dot(a, a), dot(b, b), dot(c, c)), vec3(0.));
    let n = h * h * h * h * vec3(dot(a, hash(i + 0.0)), dot(b, hash(i + o)), dot(c, hash(i + 1.0)));
    return dot(n, vec3(70.0));
}

fn history_color(tex_coords: vec2<f32>, lookup: vec4<f32>) -> vec4<f32> {
    let age = lookup.r * 255.0;
    // let offset = tex_coords + age * vec2(0.318374, 0.73492);
    let offset = tex_coords;
    let is_border = lookup.g;
    let color = gradient(age / 20.0);
    let pattern = u32(age) % 6u;
    if age > 20.0 {
        return vec4(0.0, 0.0, 0.0, 0.0);
    }
    if is_border > 0.99 {
        return vec4(mix(color, vec3(0.0, 0.0, 0.0), 0.5), 1.0);
    }
    if pattern < 2u {
        return vec4(color, 1.0);
    } else if pattern < 4u {
        let hex_center = pointy_hex_to_pixel(round(pixel_to_pointy_hex(offset)));
        let dist = length(offset - hex_center);
        if (dist < (0.5 * size)) != (pattern == 3u) {
            return vec4(0.0, 0.0, 0.0, 0.0);
        } else {
            return vec4(color, 1.0);
        }
    } else {
        let t = fract((offset.x - offset.y) / (size * 2.0 * sqrt(2.0)));
        if (t < 0.5) != (pattern == 5u) {
            return vec4(0.0, 0.0, 0.0, 0.0);
        } else {
            return vec4(color, 1.0);
        }
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

fn oklch_to_lin(oklch: vec3<f32>) -> vec3<f32> {
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

    return rgb_linear;
    // 3. linear sRGB -> non-linear sRGB
    // If I convert to sRGB here, the color of the paper doesn't match what I
    // picked. Is nannou converting later?
    // return vec3<f32>(
        // gamma_correct(rgb_linear.x),
        // gamma_correct(rgb_linear.y),
        // gamma_correct(rgb_linear.z)
    // );
}

const num_grad_stops: u32 = 4u;

fn gradient(pos: f32) -> vec3<f32> {
    var grad_stops = array<vec3<f32>, num_grad_stops>(
        vec3(0.7, 0.1135, 48.18),
        vec3(0.7, 0.1135, 130.41),
        vec3(0.7, 0.1135, 140.41),
        vec3(0.7, 0.1135, 239.82),
    );

    let pos_mul = clamp(pos, 0.0, 1.0) * f32(num_grad_stops);
    let t = fract(pos_mul);
    let i = u32(pos_mul);
    let j = min(i + 1u, num_grad_stops - 1u);
    let color = mix(grad_stops[i], grad_stops[j], t);
    return oklch_to_lin(color);
}
