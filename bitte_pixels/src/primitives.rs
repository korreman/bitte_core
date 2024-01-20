use bytemuck::{cast_slice, Pod, Zeroable};
use std::{mem::size_of, rc::Rc};
use wgpu::{
    vertex_attr_array, BlendState, Buffer, BufferAddress, BufferDescriptor, BufferUsages,
    ColorTargetState, ColorWrites, FragmentState, IndexFormat, MultisampleState,
    PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, VertexBufferLayout, VertexState, VertexStepMode,
};

#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod)]
pub struct PrimitiveVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

pub struct LineStrip {
    pub points: Vec<PrimitiveVertex>,
}

impl PrimitiveVertex {
    const LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: size_of::<Self>() as BufferAddress,
        step_mode: VertexStepMode::Vertex,
        attributes: &vertex_attr_array![0 => Float32x2, 1 => Float32x4],
    };
}

#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod)]
pub struct IndexVertex {
    idx: u32,
}

impl IndexVertex {
    const LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: size_of::<Self>() as BufferAddress,
        step_mode: VertexStepMode::Vertex,
        attributes: &vertex_attr_array![0 => Uint32],
    };
}

fn create_pipeline<const B: usize>(
    ctx: &crate::Context,
    name: &str,
    buffer_layouts: [VertexBufferLayout; B],
    topology: wgpu::PrimitiveTopology,
    index_format: Option<IndexFormat>,
) -> (wgpu::RenderPipeline, [Buffer; B]) {
    let pipeline_layout = ctx
        .device
        .create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(name),
            bind_group_layouts: &[&ctx.canvas.dimensions_layout],
            push_constant_ranges: &[],
        });

    let pipeline = ctx
        .device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(name),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &ctx.shaders,
                entry_point: "primitive_v",
                buffers: &[PrimitiveVertex::LAYOUT],
            },
            primitive: PrimitiveState {
                topology,
                strip_index_format: index_format,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &ctx.shaders,
                entry_point: "primitive_f",
                targets: &[Some(ColorTargetState {
                    format: ctx.canvas.color_format,
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });
    let buffers = buffer_layouts.map(|layout| {
        ctx.device.create_buffer(&BufferDescriptor {
            label: Some("primitives"),
            size: 0x10000 * layout.array_stride, // TODO: Make configurable?
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST | BufferUsages::INDEX,
            mapped_at_creation: false,
        })
    });
    (pipeline, buffers)
}

pub(crate) struct Renderer {
    ctx: Rc<super::Context>,
    pixel_vertices: Buffer,
    pixel_pipeline: RenderPipeline,
    linestrip_vertices: Buffer,
    linestrip_idxs: Buffer,
    linestrip_pipeline: RenderPipeline,
}

impl Renderer {
    pub(crate) fn new(ctx: Rc<super::Context>) -> Self {
        let (pixel_pipeline, [pixel_vertices]) = create_pipeline(
            ctx.as_ref(),
            "pixels",
            [PrimitiveVertex::LAYOUT],
            PrimitiveTopology::PointList,
            None,
        );

        let (linestrip_pipeline, [linestrip_idxs, linestrip_vertices]) = create_pipeline(
            ctx.as_ref(),
            "linestrips",
            [PrimitiveVertex::LAYOUT, IndexVertex::LAYOUT],
            PrimitiveTopology::LineStrip,
            Some(IndexFormat::Uint32),
        );

        Self {
            ctx,
            pixel_vertices,
            pixel_pipeline,
            linestrip_vertices,
            linestrip_idxs,
            linestrip_pipeline,
        }
    }

    pub(crate) fn render<'a>(
        &'a mut self,
        render_pass: &mut RenderPass<'a>,
        pixels: &[PrimitiveVertex],
        linestrips: &[LineStrip],
    ) {
        let queue = &self.ctx.queue;
        // Write pixels to buffer
        queue.write_buffer(&self.pixel_vertices, 0, cast_slice(pixels));

        // Draw pixels
        render_pass.set_pipeline(&self.pixel_pipeline);
        render_pass.set_vertex_buffer(0, self.pixel_vertices.slice(..));
        render_pass.draw(0..pixels.len() as u32, 0..1);

        // Gather linestrip data
        let mut vs: Vec<PrimitiveVertex> = Vec::new();
        let mut is = Vec::new();
        let mut counter: u32 = 0;
        for LineStrip { points } in linestrips {
            is.extend(counter..(counter + points.len() as u32));
            counter += points.len() as u32;
            is.push(u32::MAX);
            vs.extend(points.iter());
        }

        // Write linestrip data to buffers
        queue.write_buffer(&self.linestrip_vertices, 0, cast_slice(vs.as_slice()));
        queue.write_buffer(&self.linestrip_idxs, 0, cast_slice(is.as_slice()));

        // Draw linestrips
        render_pass.set_pipeline(&self.linestrip_pipeline);
        render_pass.set_vertex_buffer(0, self.linestrip_vertices.slice(..));
        render_pass.set_index_buffer(self.linestrip_idxs.slice(..), IndexFormat::Uint32);
        render_pass.draw_indexed(0..is.len() as u32, 0, 0..1);
    }
}
