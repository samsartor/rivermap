use nannou::color::IntoLinSrgba;
use nannou::noise::{Fbm, MultiFractal, NoiseFn, Seedable};
use nannou::prelude::*;
use nannou::wgpu::{
    BindGroup, BlendComponent, BlendFactor, BlendOperation, Buffer, RenderPipeline, Texture,
};
use std::cell::Cell;
use std::f32;
use std::time::Duration;

use crate::river::River;

mod m_1_5_03;
mod river;

static WIDTH: u32 = 720;
static HEIGHT: u32 = 720;
static F_WIDTH: f32 = WIDTH as f32;
static F_HEIGHT: f32 = HEIGHT as f32;
static F_HEIGHT_H: f32 = F_HEIGHT / 2.0;
static F_WIDTH_H: f32 = F_WIDTH / 2.0;

static SLOWDOWN: f32 = 0.0;

fn main() {
    nannou::app(model).update(update).run();
}

fn model(app: &App) -> Model {
    app.set_exit_on_escape(true);
    app.new_window()
        .size(WIDTH, HEIGHT)
        .view(view)
        // .key_released(key_released)
        .build()
        .unwrap();
    let mut model = Model::new(app);
    apply_preset(&mut model);
    model
}

fn update(_app: &App, model: &mut Model, mut update: Update) {
    update.since_last = update.since_last.min(Duration::from_millis(200));
    model.river.recompute();
    model.river.step(update, &model.heightmap);
    model.river.distribute();
    model.river.tesselate(&model.widthmap);
    model
        .time_since_last_history
        .update(|last| last + update.since_last);
}

fn view(app: &App, model: &Model, mut frame: Frame) {
    // Begin drawing
    let draw = app.draw();

    if frame.nth() == 0 || app.keys.down.contains(&Key::Delete) {
        draw.background().color(WHITE);
    } else {
        draw.rect()
            .wh(app.window_rect().wh())
            .rgba(1.0, 1.0, 1.0, 1.0);
    }
    // for x in range(-400.0, 400.0, 10.0) {
    //     for y in range(-400.0, 400.0, 10.0) {
    //         let height = model.heightmap.get(vec2(x, y));
    //         draw.rect()
    //             .wh(vec2(10.0, 10.0))
    //             .xy(vec2(x, y))
    //             .color(rgb(height, height, height));
    //     }
    // }
    model.draw(&draw, app, &mut frame);
    // for &Node {
    //     tangent,
    //     bitangent,
    //     loc,
    // } in &model.river.segments
    // {
    //     draw.arrow()
    //         .start(loc)
    //         .end(loc + tangent * 10.0)
    //         .color(BLUE);
    //     draw.arrow()
    //         .start(loc)
    //         .end(loc + bitangent * 10.0)
    //         .color(RED);
    // }

    // Write the result of our drawing to the window's frame.
    // draw.to_frame(app, &frame).unwrap();
}

#[derive(Debug)]
struct Model {
    river: river::River,
    preset: Preset,
    heightmap: Heightmap,
    widthmap: Heightmap,
    river_history: Render,
    border: Render,
    fill: Render,
    time_since_last_history: Cell<Duration>,
    bind_group: BindGroup,
    render_pipeline: RenderPipeline,
    vertex_buffer: Buffer,
}

impl Model {
    pub fn new(app: &App) -> Self {
        let river_history = Render::new(app);
        let border = Render::new(app);
        let fill = Render::new(app);
        let vs_desc = wgpu::include_wgsl!("shaders/compositor_vs.wgsl");
        let fs_desc = wgpu::include_wgsl!("shaders/compositor_fs.wgsl");
        let window = app.main_window();
        let device = window.device();
        let vs_mod = device.create_shader_module(vs_desc);
        let fs_mod = device.create_shader_module(fs_desc);

        let sampler_desc = wgpu::SamplerBuilder::new().into_descriptor();
        let sampler_filtering = wgpu::sampler_filtering(&sampler_desc);
        let sampler = device.create_sampler(&sampler_desc);
        let textures = [&river_history, &border, &fill];
        let bind_group_layout = create_bind_group_layout(device, &textures, sampler_filtering);
        let bind_group = create_bind_group(device, &bind_group_layout, &textures, &sampler);
        let pipeline_layout = create_pipeline_layout(device, &bind_group_layout);
        let render_pipeline = create_render_pipeline(
            device,
            &pipeline_layout,
            &vs_mod,
            &fs_mod,
            Frame::TEXTURE_FORMAT,
            window.msaa_samples(),
        );
        let vertices_bytes = vertices_as_bytes(&VERTICES[..]);
        let usage = wgpu::BufferUsages::VERTEX;
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: vertices_bytes,
            usage,
        });

        Model {
            river: River::default(),
            preset: Preset::default(),
            heightmap: Heightmap::new(random(), 100.0),
            widthmap: Heightmap::new(random(), 50.0),
            time_since_last_history: Cell::new(Duration::ZERO),
            river_history,
            border,
            fill,
            bind_group,
            render_pipeline,
            vertex_buffer,
        }
    }

    pub fn draw(&self, draw: &Draw, app: &App, frame: &mut Frame) {
        let history_fade = 0.001;
        let snapshot_every = 0.5;
        let snapshot_frac = self.time_since_last_history.get().as_secs_f32() / snapshot_every;
        self.river_history
            .render_frame(app, frame, |size, history| {
                if frame.nth() == 0 {
                    history.rect().wh(size).rgba(1.0, 1.0, 1.0, 1.0);
                } else {
                    history
                        .blend(BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        })
                        .rect()
                        .wh(app.main_window().rect().wh())
                        .rgba(1.0, 1.0, 1.0, history_fade);
                }
                if frame.nth() == 0 || snapshot_frac > 1.0 {
                    self.river.draw_for_history(history);
                    self.time_since_last_history.set(Duration::ZERO);
                }
            });

        // draw.texture(&self.river_history.texture);

        self.fill.render_frame(app, frame, |_, draw| {
            draw.background().rgba(0.0, 0.0, 0.0, 0.0);
            self.river.draw_fill(draw)
        });
        self.border.render_frame(app, frame, |_, draw| {
            draw.background().rgba(0.0, 0.0, 0.0, 0.0);
            self.river.draw_border(draw)
        });
        // draw.texture(&self.fill.texture);
        // draw.texture(&self.border.texture);

        let mut encoder = frame.command_encoder();
        let mut render_pass = wgpu::RenderPassBuilder::new()
            .color_attachment(frame.texture_view(), |color| color)
            .begin(&mut encoder);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        let vertex_range = 0..VERTICES.len() as u32;
        let instance_range = 0..1;
        render_pass.draw(vertex_range, instance_range);
    }
}

fn create_bind_group_layout(
    device: &wgpu::Device,
    texture_sample_type: &[&Render],
    // texture_sample_type: &[wgpu::TextureSampleType],
    sampler_filtering: bool,
) -> wgpu::BindGroupLayout {
    let mut layout_builder = wgpu::BindGroupLayoutBuilder::new();
    for texture in texture_sample_type {
        layout_builder = layout_builder.texture(
            wgpu::ShaderStages::FRAGMENT,
            false,
            wgpu::TextureViewDimension::D2,
            texture.texture.view().build().sample_type(),
        );
    }
    layout_builder
        .sampler(wgpu::ShaderStages::FRAGMENT, sampler_filtering)
        .build(device)
}

fn create_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    textures: &[&Render],
    // texture: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    let mut group_builder = wgpu::BindGroupBuilder::new();
    let built = textures
        .iter()
        .map(|r| r.texture.view().build())
        .collect::<Vec<_>>();
    for texture in &built {
        group_builder = group_builder.texture_view(texture);
    }
    group_builder.sampler(sampler).build(device, layout)
}

fn create_pipeline_layout(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::PipelineLayout {
    let desc = wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    };
    device.create_pipeline_layout(&desc)
}

fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    vs_mod: &wgpu::ShaderModule,
    fs_mod: &wgpu::ShaderModule,
    dst_format: wgpu::TextureFormat,
    sample_count: u32,
) -> wgpu::RenderPipeline {
    wgpu::RenderPipelineBuilder::from_layout(layout, vs_mod)
        .fragment_shader(fs_mod)
        .color_format(dst_format)
        .add_vertex_buffer::<Vertex>(&wgpu::vertex_attr_array![0 => Float32x2])
        .sample_count(sample_count)
        .primitive_topology(wgpu::PrimitiveTopology::TriangleStrip)
        .build(device)
}

// The vertex type that we will use to represent a point on our triangle.
#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    position: [f32; 2],
}

// The vertices that make up the rectangle to which the image will be drawn.
const VERTICES: [Vertex; 4] = [
    Vertex {
        position: [-1.0, 1.0],
    },
    Vertex {
        position: [-1.0, -1.0],
    },
    Vertex {
        position: [1.0, 1.0],
    },
    Vertex {
        position: [1.0, -1.0],
    },
];

// See the `nannou::wgpu::bytes` documentation for why this is necessary.
fn vertices_as_bytes(data: &[Vertex]) -> &[u8] {
    unsafe { wgpu::bytes::from_slice(data) }
}

#[derive(Clone, Debug)]
struct Render {
    texture: Texture,
}

impl Render {
    pub fn new(app: &App) -> Self {
        let texture = wgpu::TextureBuilder::new()
            .size([WIDTH, HEIGHT])
            // Our texture will be used as the RENDER_ATTACHMENT for our `Draw` render pass.
            // It will also be SAMPLED by the `TextureCapturer` and `TextureResizer`.
            .usage(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING)
            // Use nannou's default multisampling sample count.
            .sample_count(1)
            // Use a spacious 16-bit linear sRGBA format suitable for high quality drawing.
            .format(wgpu::TextureFormat::Rgba16Float)
            // Build it!
            .build(app.main_window().device());
        Render { texture }
    }

    fn render_frame(&self, app: &App, frame: &Frame, action: impl FnOnce(Vec2, &Draw)) {
        let window = app.main_window();
        let mut renderer = nannou::draw::RendererBuilder::new()
            .build_from_texture_descriptor(window.device(), self.texture.descriptor());
        let draw = Draw::new();
        let size = app.main_window().rect().wh();
        action(size, &draw);
        renderer.render_to_texture(
            window.device(),
            &mut frame.command_encoder(),
            &draw,
            &self.texture,
        );
    }
}

#[derive(Clone, Debug)]
struct Heightmap {
    perlin: Fbm,
    scale: f64,
}

impl Heightmap {
    fn new(seed: u32, scale: f32) -> Self {
        Heightmap {
            perlin: Fbm::new().set_octaves(6).set_seed(seed),
            scale: scale as f64,
        }
    }
    pub fn get(&self, xy: Vec2) -> f32 {
        if Heightmap::in_bounds(xy) {
            self.perlin.get((xy.as_f64() / self.scale).to_array()) as f32
        } else {
            1.0
        }
    }

    fn in_bounds(xy: Vec2) -> bool {
        xy.x < F_WIDTH_H && xy.x > -F_WIDTH_H && xy.y < F_HEIGHT_H && xy.y > -F_HEIGHT_H
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub enum Preset {
    CIRCLE,
    #[default]
    ACROSS,
}

fn apply_preset(model: &mut Model) {
    model.river.segments.clear();
    match model.preset {
        Preset::CIRCLE => {
            // model.river.closed = true;
            let smaller_side = F_WIDTH.min(F_HEIGHT);
            let radius = 0.3 * smaller_side;
            let circumference = radius * 2.0 * PI;
            let num_steps = (circumference / river::MIN_DISTANCE).ceil() as usize;
            for i in 0..num_steps {
                let theta = (i as f32 / num_steps as f32) * 2.0 * f32::consts::PI;
                let (x, y) = theta.sin_cos();
                let node = river::Node {
                    loc: vec2(x * radius, y * radius),
                    color: lin_srgba(1.0, 0.2, 0.2, 1.0),
                    ..Default::default()
                };
                if i == 0 {
                    model.river.start = node;
                } else if i == num_steps - 1 {
                    model.river.end = node;
                } else {
                    model.river.segments.push(node);
                }
            }
        }
        Preset::ACROSS => {
            model.river.closed = false;
            for i in 0..500 {
                let t = (i as f32 / 500.0) * 2.0 - 1.0;
                let x = t;
                let y = 0.1 * (t * 20.0).sin();
                let node = river::Node {
                    loc: vec2(x * F_WIDTH_H + 0.1, y * F_HEIGHT_H),
                    color: WHITE.into_lin_srgba(),
                    // color: lin_srgba(1.0, 0.2, 0.2, 1.0),
                    ..Default::default()
                };
                if i == 0 {
                    model.river.start = node;
                } else if i == 499 {
                    model.river.end = node;
                } else {
                    model.river.segments.push(node);
                }
            }
        }
    }
}

// fn range(start: f32, threshold: f32, step_size: f32) -> impl Iterator<Item = f32> {
//     std::iter::successors(Some(start), move |&prev| {
//         let next = prev + step_size;
//         (next < threshold).then_some(next)
//     })
// }
