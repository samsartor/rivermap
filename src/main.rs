use nannou::{
    noise::{NoiseFn, Perlin},
    prelude::*,
};
use std::{f32, iter::once, mem::take};

mod m_1_5_03;

static WIDTH: u32 = 720;
static HEIGHT: u32 = 720;
static F_WIDTH: f32 = WIDTH as f32;
static F_HEIGHT: f32 = HEIGHT as f32;
static F_HEIGHT_H: f32 = F_HEIGHT / 2.0;
static F_WIDTH_H: f32 = F_WIDTH / 2.0;

fn main() {
    nannou::app(model).update(update).run();
}

fn model(app: &App) -> Model {
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
        self.loc += (self.tangent * 3.0 + -self.bitangent * 7.0 + grad * 35.0)
            * update.since_last.as_secs_f32()
            * 3.0;
    }
}

#[derive(Clone, Debug, Default)]
struct River {
    start: Node,
    segments: Vec<Node>,
    end: Node,
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

    pub fn distribute(&mut self) {
        let mut new_nodes = Vec::<Node>::new();
        let mut at_loc = self.start.loc;
        let mut at_ind = 0;
        let mut distance_to_next_point = 5.0;
        let collision_distance = 5.0;
        while at_ind < self.segments.len() {
            let next_ind = match self
                .segments
                .iter()
                .enumerate()
                .skip(at_ind + 1)
                .rev()
                .filter_map(|(other_ind, other_node)| {
                    if (other_node.loc - at_loc).length_squared()
                        < collision_distance * collision_distance
                    {
                        Some(other_ind)
                    } else {
                        None
                    }
                })
                .next()
            {
                Some(found) => found,
                None => at_ind + 1,
            };
            let next_node = self.node(next_ind as isize).unwrap_or(self.end);
            let mut line = next_node.loc - at_loc;
            let mut still_to_go = line.length();
            line /= still_to_go;
            while still_to_go > 0.01 {
                let step_by = distance_to_next_point.min(still_to_go);
                let moved = line * step_by;
                at_loc += moved;
                still_to_go -= step_by;
                distance_to_next_point -= step_by;
                if distance_to_next_point <= 0.0 {
                    new_nodes.push(Node {
                        loc: at_loc,
                        ..Default::default()
                    });
                    distance_to_next_point = 5.0;
                }
            }
            at_ind = next_ind;
        }

        self.segments = new_nodes;
    }

    pub fn draw(&self, draw: &Draw) {
        let segments = self.segments.iter().copied();
        let points = once(self.start)
            .chain(segments)
            .chain(once(self.end))
            .map(|Node { loc, .. }| (loc, PINK));
        if self.closed {
            draw.polyline().weight(5.0).points_colored_closed(points);
        } else {
            draw.polyline().weight(5.0).points_colored(points);
        }
    }

    pub fn recompute(&mut self) {
        for i in 0..self.segments.len() {
            let (a, b, c) = (
                self.node(i as isize - 1).unwrap_or(self.start),
                self.segments[i],
                self.node(i as isize + 1).unwrap_or(self.end),
            );
            let (tangent, cross) = (
                (c.loc - a.loc).normalize_or_zero(),
                (b.loc - a.loc)
                    .normalize_or_zero()
                    .perp_dot((c.loc - b.loc).normalize_or_zero()),
            );
            self.segments[i].tangent = tangent;
            self.segments[i].bitangent = (tangent.perp() * cross.signum()).normalize_or_zero();
        }
        // I commented this out just to see what would happen, and the results looked good although
        // I think something janky is happening at a small scale. Feel free to uncomment and see
        // what's up.
        //
        // let mut new_bitangents = Vec::new();
        // for i in 0..self.segments.len() {
        //     let mut new_bitangent = Vec2::ZERO;
        //     let mut count = 0.0;
        //
        //     for j in -3..=3 {
        //         if let Some(Node { bitangent, .. }) = self.node(i as isize + j) {
        //             count += 1.0;
        //             new_bitangent += bitangent;
        //         }
        //     }
        //     new_bitangents.push(new_bitangent / count)
        // }
        // for (Node { bitangent, .. }, new_bitangent) in self.segments.iter_mut().zip(new_bitangents)
        // {
        //     *bitangent = new_bitangent;
        // }

        // let mut new_locs = Vec::new();
        // for i in 0..self.segments.len() {
        //     let mut new_loc = Vec2::ZERO;
        //     let mut count = 0.0;
        //
        //     for j in -2..=2 {
        //         if let Some(Node { loc, .. }) = self.node(i as isize + j) {
        //             count += 1.0;
        //             new_loc += loc;
        //         }
        //     }
        //     new_locs.push(new_loc / count)
        // }
        // for (Node { loc, .. }, new_loc) in self.segments.iter_mut().zip(new_locs) {
        //     *loc = new_loc;
        // }
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
            model.river.closed = true;
            for i in 0..500 {
                let theta = (i as f32 / 500.0) * 2.0 * f32::consts::PI;
                let (x, y) = theta.sin_cos();
                let node = Node {
                    loc: vec2(x * 0.3 * F_WIDTH, y * 0.3 * F_HEIGHT),
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
        Preset::ACROSS => {
            model.river.closed = false;
            for i in 0..500 {
                let t = (i as f32 / 500.0) * 2.0 - 1.0;
                let x = t;
                let y = 0.1 * (t * 20.0).sin();
                let node = Node {
                    loc: vec2(x * F_WIDTH_H, y * F_HEIGHT_H),
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
