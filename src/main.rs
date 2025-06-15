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
    for i in 0..100 {
        let theta = (i as f32 / 100.0) * 2.0 * f32::consts::PI;
        let (x, y) = theta.sin_cos();
        model.river.segments.push(Node {
            loc: vec2((x * 0.3 + 0.5) * 720.0, (y * 0.3 + 0.5) * 720.0),
        })
    }
    model
}

pub fn update(_app: &App, model: &mut Model, _update: Update) {
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

    // Write the result of our drawing to the window's frame.
    draw.to_frame(app, &frame).unwrap();
}

#[derive(Copy, Clone, Debug, Default)]
struct Node {
    loc: Vec2,
}

#[derive(Clone, Debug, Default)]
struct River {
    segments: Vec<Node>,
}

#[derive(Clone, Debug, Default)]
struct Model {
    river: River,
    // TODO
}
