use nannou::prelude::*;
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

pub fn update(_app: &App, model: &mut Model, _update: Update) {
    model.river.recompute();
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
        draw.background().color(BLUE);
    } else {
        draw.rect()
            .wh(app.window_rect().wh())
            .rgba(1.0, 1.0, 1.0, 0.01);
    }
    model.draw(&draw);
    for &Node {
        tangent,
        bitangent,
        loc,
    } in &model.river.segments
    {
        draw.arrow()
            .start(loc)
            .end(loc + tangent * 40.0)
            .color(BLUE);
        draw.arrow()
            .start(loc)
            .end(loc + bitangent * 40.0)
            .color(RED);
    }

    // Write the result of our drawing to the window's frame.
    draw.to_frame(app, &frame).unwrap();
}

#[derive(Copy, Clone, Debug, Default)]
struct Node {
    loc: Vec2,
    tangent: Vec2,
    bitangent: Vec2,
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
            self.segments[i].bitangent = (tangent.perp() * cross).normalize_or_zero();
        }
    }
}

#[derive(Clone, Debug, Default)]
struct Model {
    river: River,
    preset: Preset,
    // TODO
}

impl Model {
    pub fn draw(&self, draw: &Draw) {
        self.river.draw(draw);
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
            for i in 0..100 {
                let theta = (i as f32 / 100.0) * 2.0 * f32::consts::PI;
                let (x, y) = theta.sin_cos();
                model.river.segments.push(Node {
                    loc: vec2(x * 0.3 * 720.0, y * 0.3 * 720.0),
                    ..Default::default()
                })
            }
        }
    }
}
