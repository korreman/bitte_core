use log::debug;
use std::rc::Rc;
use wgpu::util::DeviceExt;

use super::{Size, Vertex};

fn calculate_active_quad(surface: &Size, internal: &Size) -> [Vertex; 4] {
    let int_scale = std::cmp::min(
        surface.width / internal.width,
        surface.height / internal.height,
    );

    let upscaled = Size {
        width: internal.width * int_scale,
        height: internal.height * int_scale,
    };

    debug!(
        "Upscale calculation.\nSurface: {:?}\nUpscale integer: {:?}\nUpscaled upscale: {:?}",
        surface, int_scale, upscaled
    );

    let x_offset = (surface.width - upscaled.width) / 2;
    let x_padding = upscaled.width + x_offset;
    let x1 = x_offset as f32 / surface.width as f32 * 2. - 1.;
    let x2 = x_padding as f32 / surface.width as f32 * 2. - 1.;

    // Note that the Y-axis is flipped here.
    // The entire image is drawn-up upside-down, then flipped around at the end.
    // This allows us to use a positive Y-axis in the renderer.
    let y_offset = (surface.height - upscaled.height) / 2;
    let y_padding = upscaled.height + y_offset;
    let y1 = y_padding as f32 / surface.height as f32 * 2. - 1.;
    let y2 = y_offset as f32 / surface.height as f32 * 2. - 1.;

    [
        Vertex { x: x1, y: y1 },
        Vertex { x: x2, y: y1 },
        Vertex { x: x1, y: y2 },
        Vertex { x: x2, y: y2 },
    ]
}

pub struct Renderer {
    ctx: Rc<super::Context>,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    active_quad_buffer: wgpu::Buffer,
    _sampler_nearest: wgpu::Sampler,
}

impl Renderer {
    pub(crate) fn new(ctx: Rc<super::Context>) -> Self {
        let device = &ctx.device;
        let active_quad_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("canvas quad buffer"),
            contents: bytemuck::cast_slice(&calculate_active_quad(
                &ctx.canvas.size,
                &ctx.canvas.size,
            )),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("canvas to surface bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("canvas to surface pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/upscale.wgsl"));

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("upscale to surface pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "upscale_v",
                buffers: &[super::QUAD_LAYOUT, super::TEXCOORD_LAYOUT],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "upscale_f",
                targets: &[Some(wgpu::ColorTargetState {
                    format: ctx.canvas.color_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::all(),
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                .. Default::default()
            },
            multisample: wgpu::MultisampleState::default(),
            depth_stencil: None,
            multiview: None,
        });

        let sampler_nearest = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("upscale sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("upscale bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&ctx.canvas.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler_nearest),
                },
            ],
        });

        Self {
            ctx,
            pipeline,
            bind_group,
            active_quad_buffer,
            _sampler_nearest: sampler_nearest,
        }
    }

    pub fn renew_active_quad(&self, queue: &wgpu::Queue, surface_size: Size) {
        queue.write_buffer(
            &self.active_quad_buffer,
            0,
            bytemuck::cast_slice(&calculate_active_quad(
                &surface_size,
                &self.ctx.canvas.size,
            )),
        );
    }

    pub(crate) fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target_surface: &wgpu::TextureView,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("upscale render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_surface,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None, // TODO: Check this
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.active_quad_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.ctx.quad_buffer.slice(..));
        render_pass.draw(0..super::QUAD_VERTICES.len() as u32, 0..1);
    }
}
