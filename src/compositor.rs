use nannou::{
    App, Frame,
    prelude::DeviceExt,
    wgpu::{self, BindGroup, Buffer, BufferInitDescriptor, RenderPipeline},
};

use crate::Render;

#[derive(Debug)]
pub struct Compositor {
    bind_group: BindGroup,
    render_pipeline: RenderPipeline,
    vertex_buffer: Buffer,
}

impl Compositor {
    pub fn new(app: &App, textures: &[&Render]) -> Self {
        let vs_desc = wgpu::include_wgsl!("shaders/compositor_vs.wgsl");
        let fs_desc = wgpu::include_wgsl!("shaders/compositor_fs.wgsl");
        let window = app.main_window();
        let device = window.device();
        let vs_mod = device.create_shader_module(vs_desc);
        let fs_mod = device.create_shader_module(fs_desc);

        let sampler_desc = wgpu::SamplerBuilder::new()
            .mag_filter(wgpu::FilterMode::Nearest)
            .min_filter(wgpu::FilterMode::Nearest)
            .into_descriptor();
        let sampler_filtering = wgpu::sampler_filtering(&sampler_desc);
        let sampler = device.create_sampler(&sampler_desc);
        let bind_group_layout = create_bind_group_layout(device, textures, sampler_filtering);
        let bind_group = create_bind_group(device, &bind_group_layout, textures, &sampler);
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

        Compositor {
            bind_group,
            render_pipeline,
            vertex_buffer,
        }
    }

    pub fn draw(&self, frame: &Frame) {
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
