use bytemuck::{cast_slice, Pod, Zeroable};
use std::{mem::size_of, rc::Rc};
use wgpu::{
    vertex_attr_array, BlendState, Buffer, BufferAddress, BufferDescriptor, BufferUsages,
    ColorTargetState, ColorWrites, FragmentState, MultisampleState, PipelineLayoutDescriptor,
    PrimitiveState, PrimitiveTopology, RenderPass, RenderPipeline, RenderPipelineDescriptor,
    VertexBufferLayout, VertexState, VertexStepMode,
};

/// Sent to the shader for rendering.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Circle {
    /// Position on the canvas
    pub offset: [i32; 2],
    /// Width and height in pixels.
    pub diameter: u32,
    /// Zero-indexed position in the sheet.
    pub color: [f32; 4],
}

impl Circle {
    const LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: size_of::<Self>() as BufferAddress,
        step_mode: VertexStepMode::Instance,
        attributes: &vertex_attr_array![1 => Sint32x2, 2 => Uint32x2, 3 => Float32x4],
    };
}

pub(crate) struct Renderer {
    ctx: Rc<super::Context>,
    pipeline: RenderPipeline,
    instance_buffer: Buffer,
}

impl Renderer {
    pub fn new(ctx: Rc<super::Context>) -> Self {
        const MAX_INSTANCES: u64 = 2048;
        let device = &ctx.device;

        let instance_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("circle instances"),
            size: MAX_INSTANCES * size_of::<Circle>() as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("circle"),
            bind_group_layouts: &[&ctx.canvas.dimensions_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("circle"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &ctx.shaders,
                entry_point: "circle_v",
                buffers: &[super::QUAD_LAYOUT, Circle::LAYOUT],
            },
            fragment: Some(FragmentState {
                module: &ctx.shaders,
                entry_point: "circle_f",
                targets: &[Some(ColorTargetState {
                    format: ctx.canvas.color_format,
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: ColorWrites::all(),
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            multisample: MultisampleState::default(),
            depth_stencil: None,
            multiview: None,
        });

        Self {
            ctx,
            pipeline,
            instance_buffer,
        }
    }

    pub fn render<'a>(&'a mut self, render_pass: &mut RenderPass<'a>, circles: &[Circle]) {
        self.ctx
            .queue
            .write_buffer(&self.instance_buffer, 0, cast_slice(circles));

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.ctx.quad_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.draw(
            0..super::QUAD_VERTICES.len() as u32,
            0..circles.len() as u32,
        );
    }
}
