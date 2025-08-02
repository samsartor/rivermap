use nannou::{
    prelude::*,
    wgpu::{Texture, TextureBuilder},
};

use crate::{HEIGHT, WIDTH};

#[derive(Clone, Debug)]
pub struct Render {
    pub texture: Texture,
}

impl Render {
    pub fn new(app: &App) -> Self {
        let texture = TextureBuilder::new()
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

    pub fn render_frame(&self, app: &App, frame: &Frame, action: impl FnOnce(Vec2, &Draw)) {
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
