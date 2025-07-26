use nannou::{
    noise::{NoiseFn, Perlin},
    prelude::*,
};
use std::f32;

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
    let mut model = Model::default();
    apply_preset(&mut model);
    model
}

fn update(_app: &App, model: &mut Model, update: Update) {
    model.river.recompute();
    model.river.step(update, &model.heightmap);
    model.river.distribute();
}

fn view(app: &App, model: &Model, frame: Frame) {
    // Begin drawing
    let draw = app.draw();

    if frame.nth() == 0 || app.keys.down.contains(&Key::Delete) {
        draw.background().color(WHITE);
    } else {
        draw.rect()
            .wh(app.window_rect().wh())
            .rgba(1.0, 1.0, 1.0, 0.01);
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
    model.draw(&draw);
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

#[derive(Clone, Debug, Default)]
struct Model {
    river: river::River,
    preset: Preset,
    heightmap: Heightmap,
}

impl Model {
    pub fn draw(&self, draw: &Draw) {
        self.river.draw_dumb(draw);
    }
}

#[derive(Copy, Clone, Debug, Default)]
struct Heightmap {
    perlin: Perlin,
}

impl Heightmap {
    pub fn get(&self, xy: Vec2) -> f32 {
        if Heightmap::in_bounds(xy) {
            self.perlin.get((xy.as_f64() / 100.0).to_array()) as f32
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
                    width: 10.0,
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
                    loc: vec2(x * F_WIDTH_H, y * F_HEIGHT_H),
                    width: 10.0,
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
