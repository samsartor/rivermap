use nannou::color::IntoLinSrgba;
use nannou::noise::{Fbm, MultiFractal, NoiseFn, Seedable};
use nannou::prelude::*;
use nannou::wgpu::Texture;
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
    let main_window = app
        .new_window()
        .size(WIDTH, HEIGHT)
        .view(view)
        // .key_released(key_released)
        .build()
        .unwrap();
    let window = app.window(main_window).unwrap();
    let mut model = Model::new(&window);
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
    model.draw(&draw, &app.window(model.window_id).unwrap(), &mut frame);
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
    draw.to_frame(app, &frame).unwrap();
}

#[derive(Clone, Debug)]
struct Model {
    river: river::River,
    preset: Preset,
    heightmap: Heightmap,
    widthmap: Heightmap,
    window_id: WindowId,
    river_history: Texture,
    time_since_last_history: Cell<Duration>,
}

impl Model {
    pub fn new(window: &Window) -> Self {
        let sample_count = 1;
        let river_history = wgpu::TextureBuilder::new()
            .size([WIDTH, HEIGHT])
            // Our texture will be used as the RENDER_ATTACHMENT for our `Draw` render pass.
            // It will also be SAMPLED by the `TextureCapturer` and `TextureResizer`.
            .usage(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING)
            // Use nannou's default multisampling sample count.
            .sample_count(sample_count)
            // Use a spacious 16-bit linear sRGBA format suitable for high quality drawing.
            .format(wgpu::TextureFormat::Rgba16Float)
            // Build it!
            .build(window.device());
        Model {
            river: River::default(),
            preset: Preset::default(),
            heightmap: Heightmap::new(random(), 100.0),
            widthmap: Heightmap::new(random(), 50.0),
            window_id: window.id(),
            time_since_last_history: Cell::new(Duration::ZERO),
            river_history,
        }
    }
    pub fn draw(&self, draw: &Draw, window: &Window, frame: &mut Frame) {
        let descriptor = self.river_history.descriptor();
        let mut renderer = nannou::draw::RendererBuilder::new()
            .build_from_texture_descriptor(window.device(), descriptor);
        let history = Draw::new();
        if frame.nth() == 0 {
            history
                .rect()
                .wh(window.rect().wh())
                .rgba(1.0, 1.0, 1.0, 1.0);
        } else {
            history
                .rect()
                .wh(window.rect().wh())
                .rgba(1.0, 1.0, 1.0, 0.001);
        }
        if self.time_since_last_history.get().as_secs_f32() > 0.5 {
            self.river.draw_for_history(&history);
            self.time_since_last_history.set(Duration::ZERO);
        }
        renderer.render_to_texture(
            window.device(),
            &mut frame.command_encoder(),
            &history,
            &self.river_history,
        );

        draw.texture(&self.river_history);

        self.river.draw_fill(draw);
        self.river.draw_border(draw);
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

fn range(start: f32, threshold: f32, step_size: f32) -> impl Iterator<Item = f32> {
    std::iter::successors(Some(start), move |&prev| {
        let next = prev + step_size;
        (next < threshold).then_some(next)
    })
}
