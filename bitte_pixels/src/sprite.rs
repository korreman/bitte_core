use std::{num::NonZeroU32, rc::Rc};
use wgpu::util::DeviceExt;

/// Sprite data to submit for drawing.
#[derive(Clone)]
pub struct SpriteInstance {
    /// Position on the target canvas.
    pub position: [i32; 2],
    /// Index/identifier in the sprite sheet.
    pub sprite: SpriteHandle,
}

/// Sent to the shader for rendering.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceData {
    /// Position on the canvas
    position: [i32; 2],
    /// Width and height in pixels.
    dimensions: [u32; 2],
    /// Zero-indexed position in the sheet.
    sheet_position: u32,
}

const INSTANCE_LAYOUT: wgpu::VertexBufferLayout = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<InstanceData>() as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![
        1 => Sint32x2,
        2 => Uint32x2,
        3 => Uint32,
    ],
};

pub(crate) struct Renderer {
    ctx: Rc<super::Context>,
    sprite_sheet_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    instance_buffer: wgpu::Buffer,
}

impl Renderer {
    pub fn new(ctx: Rc<super::Context>) -> Self {
        const MAX_INSTANCES: u64 = 2048;
        let device = &ctx.device;

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite instances"),
            size: MAX_INSTANCES * std::mem::size_of::<InstanceData>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sprite_sheet_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("sprite sheet"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                }],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite"),
            bind_group_layouts: &[&ctx.canvas.dimensions_layout, &sprite_sheet_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &ctx.shaders,
                entry_point: "sprite_v",
                buffers: &[super::QUAD_LAYOUT, INSTANCE_LAYOUT],
            },
            fragment: Some(wgpu::FragmentState {
                module: &ctx.shaders,
                entry_point: "sprite_f",
                targets: &[Some(wgpu::ColorTargetState {
                    format: ctx.canvas.color_format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::all(),
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            multisample: wgpu::MultisampleState::default(),
            depth_stencil: None,
            multiview: None,
        });

        Self {
            ctx,
            sprite_sheet_layout,
            pipeline,
            instance_buffer,
        }
    }

    pub fn create_sprite_sheet_builder<'a>(&'a self, name: &'a str) -> SpriteSheetBuilder<'a> {
        let mut res = SpriteSheetBuilder {
            name,
            context: self.ctx.as_ref(),
            layout: &self.sprite_sheet_layout,
            table: Vec::new(),
            pixel_count: 0,
            data: Vec::new(),
        };
        // Add a dummy sprite to prevent zero-size buffer errors
        res.add(SpriteData {
            dimensions: (NonZeroU32::new(1).unwrap(), NonZeroU32::new(1).unwrap()),
            offset: (0, 0),
            data: vec![0, 0, 0, 0],
            pixels: 1,
        });
        res
    }

    pub fn render<'a>(
        &'a mut self,
        render_pass: &mut wgpu::RenderPass<'a>,
        sprite_sheet: &'a SpriteSheet,
        sprites: &[SpriteInstance],
    ) {
        let instances: Box<[InstanceData]> = sprites
            .iter()
            .map(|sprite| {
                let entry = &sprite_sheet.table[sprite.sprite.0];
                let position = [
                    sprite.position[0] + entry.offset.0,
                    sprite.position[1] + entry.offset.1,
                ];
                InstanceData {
                    position,
                    sheet_position: entry.address,
                    dimensions: [entry.dimensions.0.into(), entry.dimensions.1.into()],
                }
            })
            .collect();
        self.ctx.queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(instances.as_ref()),
        );

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(1, &sprite_sheet.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.ctx.quad_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.draw(
            0..super::QUAD_VERTICES.len() as u32,
            0..sprites.len() as u32,
        );
    }
}

// TODO: Figure out whether unused variables can be dropped.
pub struct SpriteSheet {
    /// Sprites are __not__ stored as packed boxes in a 2-dimensional grid,
    /// (as is the usual approach).
    /// They are stored as a 1-dimensional sequence to improve space usage.
    /// We can do this because we ditch the sampler entirely,
    /// instead indexing the texture with integer coordinates to get unfiltered pixels.
    /// However, 1-dimensional textures are limited to 2048 pixels (WebGL2),
    /// so this sequence is stored in a 2-dimensional texture.
    /// This brings the maximum pixel count per sprite sheet up to 4 million.
    _texture: wgpu::Texture,
    _texture_view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    table: Box<[SpriteEntry]>,
}

/// Sprite sheet data, for use with [crate::Renderer::create_sprite_sheet].
pub struct SpriteSheetBuilder<'a> {
    /// Name for debugging messages.
    name: &'a str,

    /// Render context.
    /// We use limits, device, and queue here.
    context: &'a super::Context,

    /// Bindgroup layout, which for some reason cannot be constant.
    layout: &'a wgpu::BindGroupLayout,

    /// Entry lookup table.
    table: Vec<SpriteEntry>,

    /// Current amount of pixels.
    pixel_count: u32,

    /// Image data.
    ///
    /// Sprite image data is __one-dimensional__.
    /// Given a sprite `address` and a coordinate `(x, y)`,
    /// the sampling coordinate can be computed by `address + x + y * width`.
    ///
    /// We lose linear filtering this way,
    /// but we get to avoid the 2D box packing problem.
    /// We aren't interested in linear filtering anyway.
    data: Vec<u8>,
}

impl<'a> SpriteSheetBuilder<'a> {
    fn push_sprite(&mut self, mut sprite: SpriteData) {
        self.table.push(SpriteEntry {
            address: self.pixel_count,
            dimensions: sprite.dimensions,
            offset: sprite.offset,
        });
        self.data.append(&mut sprite.data);
        self.pixel_count += sprite.pixels;
    }

    pub fn add(&mut self, sprite: SpriteData) -> SpriteHandle {
        self.push_sprite(sprite);
        SpriteHandle(self.table.len() - 1)
    }

    pub fn add_animation(&mut self, sprites: Vec<SpriteData>) -> AnimationHandle {
        let offset = self.table.len();
        for sprite in sprites {
            self.push_sprite(sprite);
        }
        let end = self.table.len();
        AnimationHandle {
            offset,
            frame_count: end - offset,
        }
    }

    pub fn build(mut self) -> SpriteSheet {
        let device = &self.context.device;

        let max_width = self.context.limits.max_texture_dimension_2d;
        let width = self.pixel_count.min(max_width);
        let height = 1 + self.pixel_count / max_width;
        info!(
            "Sprite sheet {:?} dimensions are: {width}x{height}",
            self.name
        );

        // Pad the texture data to match the exact dimensions of the texture.
        let padding = (width * height - self.pixel_count) as usize;
        self.data.extend(std::iter::repeat(0).take(padding * 4));

        let texture = device.create_texture_with_data(
            &self.context.queue,
            &wgpu::TextureDescriptor {
                label: Some(self.name),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
            },
            wgpu::util::TextureDataOrder::default(),
            self.data.as_ref(),
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(self.name),
            layout: self.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            }],
        });

        SpriteSheet {
            _texture: texture,
            _texture_view: texture_view,
            bind_group,
            table: self.table.into_boxed_slice(),
        }
    }
}

struct SpriteEntry {
    address: u32,
    dimensions: (NonZeroU32, NonZeroU32),
    offset: (i32, i32),
}

/// Sprite data for submitting to the sheet.
pub struct SpriteData {
    /// Sprite width and height.
    dimensions: (NonZeroU32, NonZeroU32),
    /// Offset relative to drawing coordinate.
    /// If (0, 0), the bottom-left corner is fixed to the coordinate.
    offset: (i32, i32),
    /// Sprite data, in Rgba8UnormSrgb, top-to-bottom left-to-right.
    data: Vec<u8>,
    /// Number of pixels in sprite
    pixels: u32,
}

impl SpriteData {
    /// Create a new sprite instance for submission to a sprite sheet.
    /// `data is given in `Rgba8UnormSrgb`, and must match in size with `dimensions`.
    pub fn new(dimensions: (u32, u32), offset: (i32, i32), data: Vec<u8>) -> Option<Self> {
        let dimensions = (
            NonZeroU32::new(dimensions.0)?,
            NonZeroU32::new(dimensions.1)?,
        );
        let pixels = dimensions.0.get() * dimensions.1.get();
        if (pixels * 4) as usize == data.len() {
            Some(Self {
                dimensions,
                offset,
                data,
                pixels,
            })
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
pub struct SpriteHandle(usize);

#[derive(Clone, Copy)]
pub struct AnimationHandle {
    offset: usize,
    frame_count: usize,
}

impl AnimationHandle {
    pub fn get_frame(&self, time: usize) -> SpriteHandle {
        SpriteHandle(self.offset + (time % self.frame_count))
    }
}
