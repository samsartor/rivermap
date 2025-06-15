use nannou::{
    noise::{NoiseFn, Perlin},
    prelude::*,
};
use std::f32;

mod m_1_5_03;

fn main() {
    nannou::app(model).update(update).run();
}

fn model(app: &App) -> Model {
    app.new_window()
        .size(720, 720)
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
    // let noise = Perlin::new().set_seed(model.noise_seed);
    //
    // for agent in &mut model.agents {
    //     match model.draw_mode {
    //         1 => agent.update1(noise, model.noise_scale, model.noise_strength),
    //         2 => agent.update2(noise, model.noise_scale, model.noise_strength),
    //         _ => (),
    //     }
    //     agent.update(model.noise_z_velocity);
    // }
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

#[derive(Copy, Clone, Debug, Default)]
struct Node {
    loc: Vec2,
    tangent: Vec2,
    bitangent: Vec2,
}

impl Node {
    pub fn step(&mut self, update: Update, heightmap: &Heightmap) {
        let up = heightmap.get(self.loc + vec2(1.0, 0.0));
        let down = heightmap.get(self.loc + vec2(-1.0, 0.0));
        let left = heightmap.get(self.loc + vec2(0.0, -1.0));
        let right = heightmap.get(self.loc + vec2(0.0, 1.0));
        let grad = -vec2(up - down, right - left);
        self.loc += (self.tangent * 3.0 + -self.bitangent * 2.0 + grad * 10.0)
            // * update.since_last.as_secs_f32()
            * 1.0;
    }
}

#[derive(Clone, Debug, Default)]
struct River {
    segments: Vec<Node>,
    closed: bool,
}

impl River {
    pub fn node(&self, i: isize) -> Option<Node> {
        if self.closed {
            Some(self.segments[i.rem_euclid(self.segments.len() as isize) as usize])
        } else if i > 0 {
            self.segments.get(i as usize).copied()
        } else {
            None
        }
    }

    pub fn draw(&self, draw: &Draw) {
        let points = self
            .segments
            .iter()
            .copied()
            .map(|Node { loc, .. }| (loc, PINK));
        if self.closed {
            draw.polyline().weight(5.0).points_colored_closed(points);
        } else {
            draw.polyline().weight(5.0).points_colored(points);
        }
    }

    pub fn recompute(&mut self) {
        for i in 0..self.segments.len() {
            let (tangent, cross) = match (
                self.node(i as isize - 1),
                self.segments[i],
                self.node(i as isize + 1),
            ) {
                (None, _, None) => panic!("Nope!"),
                (None, b, Some(c)) => ((c.loc - b.loc).normalize_or_zero(), 0.0),
                (Some(a), b, None) => ((b.loc - a.loc).normalize_or_zero(), 0.0),
                (Some(a), b, Some(c)) => (
                    (c.loc - a.loc).normalize_or_zero(),
                    (b.loc - a.loc)
                        .normalize_or_zero()
                        .perp_dot((c.loc - b.loc).normalize_or_zero()),
                ),
            };
            self.segments[i].tangent = tangent;
            self.segments[i].bitangent = (tangent.perp() * cross.signum()).normalize_or_zero();
        }
        let mut new_bitangents = Vec::new();
        for i in 0..self.segments.len() {
            let mut new_bitangent = Vec2::ZERO;
            let mut count = 0.0;

            for j in -1..=1 {
                if let Some(Node { bitangent, .. }) = self.node(i as isize + j) {
                    count += 1.0;
                    new_bitangent += bitangent;
                }
            }
            new_bitangents.push(new_bitangent / count)
        }
        for (Node { bitangent, .. }, new_bitangent) in self.segments.iter_mut().zip(new_bitangents)
        {
            *bitangent = new_bitangent;
        }

        let mut new_locs = Vec::new();
        for i in 0..self.segments.len() {
            let mut new_loc = Vec2::ZERO;
            let mut count = 0.0;

            for j in -2..=2 {
                if let Some(Node { loc, .. }) = self.node(i as isize + j) {
                    count += 1.0;
                    new_loc += loc;
                }
            }
            new_locs.push(new_loc / count)
        }
        for (Node { loc, .. }, new_loc) in self.segments.iter_mut().zip(new_locs) {
            *loc = new_loc;
        }
    }

    pub fn step(&mut self, update: Update, heightmap: &Heightmap) {
        for node in &mut self.segments {
            node.step(update, heightmap);
        }
    }
}

#[derive(Clone, Debug, Default)]
struct Model {
    river: River,
    preset: Preset,
    heightmap: Heightmap,
}

impl Model {
    pub fn draw(&self, draw: &Draw) {
        self.river.draw(draw);
    }
}

#[derive(Copy, Clone, Debug, Default)]
struct Heightmap {
    perlin: Perlin,
}

impl Heightmap {
    pub fn get(&self, xy: Vec2) -> f32 {
        self.perlin.get((xy.as_f64() / 100.0).to_array()) as f32
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub enum Preset {
    #[default]
    CIRCLE,
}

fn apply_preset(model: &mut Model) {
    model.river.segments.clear();
    match model.preset {
        Preset::CIRCLE => {
            model.river.closed = true;
            for i in 0..500 {
                let theta = (i as f32 / 500.0) * 2.0 * f32::consts::PI;
                let (x, y) = theta.sin_cos();
                model.river.segments.push(Node {
                    loc: vec2(x * 0.3 * 720.0, y * 0.3 * 720.0),
                    ..Default::default()
                })
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
