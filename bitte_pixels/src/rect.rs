use std::rc::Rc;
use wgpu::util::DeviceExt;

use super::Vertex;

const LINEBOX_VERTICES: &[Vertex; 5] = &[
    Vertex { x: 0.0, y: 0.0 },
    Vertex { x: 1.0, y: 0.0 },
    Vertex { x: 1.0, y: 1.0 },
    Vertex { x: 0.0, y: 1.0 },
    Vertex { x: 0.0, y: 0.0 },
];

const LINEBOX_LAYOUT: wgpu::VertexBufferLayout = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Rectangle {
    pub position: [i32; 2],
    pub dimensions: [u32; 2],
    pub color: [f32; 4],
}

const RECTANGLE_LAYOUT: wgpu::VertexBufferLayout = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<Rectangle>() as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![1 => Sint32x2, 2 => Uint32x2, 3 => Float32x4],
};

pub(crate) struct Renderer {
    ctx: Rc<super::Context>,
    pipeline: wgpu::RenderPipeline,
    linebox_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
}

impl Renderer {
    pub(crate) fn new(ctx: Rc<super::Context>) -> Self {
        const MAX_INSTANCES: u64 = 2048;
        let device = &ctx.device;

        let linebox_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rect vertices"),
            contents: bytemuck::cast_slice(LINEBOX_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rect instaces"),
            size: MAX_INSTANCES * std::mem::size_of::<Rectangle>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rect"),
            bind_group_layouts: &[&ctx.canvas.dimensions_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rect"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &ctx.shaders,
                entry_point: "rect_v",
                buffers: &[LINEBOX_LAYOUT, RECTANGLE_LAYOUT],
            },
            fragment: Some(wgpu::FragmentState {
                module: &ctx.shaders,
                entry_point: "rect_f",
                targets: &[Some(wgpu::ColorTargetState {
                    format: ctx.canvas.color_format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self {
            ctx,
            pipeline,
            linebox_buffer,
            instance_buffer,
        }
    }

    // Write the rectangles to the instance buffer
    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        rectangles: &[Rectangle],
    ) {
        self.ctx.queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(rectangles));
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.linebox_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.draw(0..LINEBOX_VERTICES.len() as u32, 0..rectangles.len() as u32);
    }
}
