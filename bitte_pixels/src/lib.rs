use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use wgpu::{util::DeviceExt, StoreOp};
#[macro_use]
extern crate log;

mod circles;
mod primitives;
mod rect;
pub mod sprite;
mod upscale;

pub use circles::Circle;
pub use primitives::{LineStrip, PrimitiveVertex};
pub use rect::Rectangle;
use sprite::{SpriteInstance, SpriteSheet, SpriteSheetBuilder};

// Buffer element types and constants
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    x: f32,
    y: f32,
}

const QUAD_LAYOUT: wgpu::VertexBufferLayout = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
};

const TEXCOORD_LAYOUT: wgpu::VertexBufferLayout = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &wgpu::vertex_attr_array![1 => Float32x2],
};

#[rustfmt::skip]
const QUAD_VERTICES: &[Vertex; 4] = &[
    Vertex { x: 0., y: 0. }, Vertex { x: 1., y: 0. },
    Vertex { x: 0., y: 1. }, Vertex { x: 1., y: 1. },
];

// Renderer components
pub struct Scene {
    pub pixels: Vec<PrimitiveVertex>,
    pub linestrips: Vec<LineStrip>,
    pub circles: Vec<Circle>,
    pub rectangles: Vec<Rectangle>,
    pub sprites: Vec<SpriteInstance>,
}

#[derive(std::fmt::Debug, Clone, Copy)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub enum RenderError {
    CreateSurface,
    AcquireAdapter,
    AcquireDevice,
    SurfaceTexture,
    Other(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err = match self {
            RenderError::CreateSurface => "failed to create surface",
            RenderError::AcquireAdapter => "adapter request failed",
            RenderError::AcquireDevice => "device request failed",
            RenderError::SurfaceTexture => "couldn't acquire surface texture",
            RenderError::Other(err) => err,
        };
        f.write_str(err)
    }
}

pub struct Renderer<'w> {
    ctx: std::rc::Rc<Context>,
    surface: wgpu::Surface<'w>,
    surface_config: wgpu::SurfaceConfiguration,
    rect_renderer: rect::Renderer,
    sprite_renderer: sprite::Renderer,
    upscale_renderer: upscale::Renderer,
    primitives_renderer: primitives::Renderer,
    circle_renderer: circles::Renderer,
}

impl<'w> Renderer<'w> {
    pub async fn new<S>(
        window: S,
        window_size: Size,
        game_resolution: Size,
    ) -> Result<Self, RenderError>
    where
        S: 'w + HasDisplayHandle + HasWindowHandle + Send + Sync,
    {
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window)
            .map_err(|_| RenderError::CreateSurface)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(RenderError::AcquireAdapter)?;

        let mut limits = wgpu::Limits::downlevel_webgl2_defaults();
        limits.max_texture_dimension_2d = 8192;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_limits: limits.clone(),
                    ..Default::default()
                },
                None,
            )
            .await
            .map_err(|_| RenderError::AcquireDevice)?;

        let screen_color_format = *surface
            .get_capabilities(&adapter)
            .formats
            .get(0)
            .unwrap_or(&wgpu::TextureFormat::Rgba8UnormSrgb);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: screen_color_format,
            width: window_size.width,
            height: window_size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![screen_color_format],
            desired_maximum_frame_latency: 1,
        };

        surface.configure(&device, &surface_config);

        let quad_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad buffer"),
            contents: bytemuck::cast_slice(QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let canvas = Canvas::new(&device, "final image", game_resolution, screen_color_format);

        let shaders = device.create_shader_module(wgpu::include_wgsl!("shaders/main.wgsl"));

        let ctx = std::rc::Rc::new(Context {
            device,
            queue,
            limits,
            shaders,
            canvas,
            quad_buffer,
        });

        let primitives_renderer = primitives::Renderer::new(ctx.clone());
        let circle_renderer = circles::Renderer::new(ctx.clone());
        let rect_renderer = rect::Renderer::new(ctx.clone());
        let sprite_renderer = sprite::Renderer::new(ctx.clone());
        let upscale_renderer = upscale::Renderer::new(ctx.clone());

        Ok(Self {
            ctx,
            surface,
            surface_config,

            primitives_renderer,
            circle_renderer,
            rect_renderer,
            sprite_renderer,
            upscale_renderer,
        })
    }

    pub fn create_sprite_sheet_builder<'a>(&'a mut self, name: &'a str) -> SpriteSheetBuilder<'a> {
        self.sprite_renderer.create_sprite_sheet_builder(name)
    }

    pub fn resize_surface(&mut self, size: Size) {
        self.surface_config.width = size.width;
        self.surface_config.height = size.height;
        self.surface
            .configure(&self.ctx.device, &self.surface_config);
        self.upscale_renderer
            .renew_active_quad(&self.ctx.queue, size);
    }

    /// Acquire the next swap chain frame.
    /// If the swap chain has been lost,
    /// this function will recreate it.
    fn get_surface_texture(&mut self) -> Result<wgpu::SurfaceTexture, RenderError> {
        match self.surface.get_current_texture() {
            Ok(frame) => Ok(frame),
            _ => {
                info!("Couldn't get swapchain surface texture, reconfiguring.");
                self.surface
                    .configure(&self.ctx.device, &self.surface_config);
                self.surface
                    .get_current_texture()
                    .map_err(|_| RenderError::SurfaceTexture)
            }
        }
    }

    pub fn render(&mut self, sprite_sheet: &SpriteSheet, scene: &Scene) -> Result<(), RenderError> {
        // Create a command encoder
        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("full pass encoder"),
            });

        // Draw items to canvas
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("fill background"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.ctx.canvas.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None, // TODO: Check this
        });
        render_pass.set_bind_group(0, &self.ctx.canvas.dimensions_bind_group, &[]);
        self.sprite_renderer
            .render(&mut render_pass, sprite_sheet, scene.sprites.as_slice());
        self.rect_renderer
            .render(&mut render_pass, scene.rectangles.as_slice());
        self.primitives_renderer.render(
            &mut render_pass,
            scene.pixels.as_slice(),
            scene.linestrips.as_slice(),
        );
        self.circle_renderer
            .render(&mut render_pass, scene.circles.as_slice());
        drop(render_pass);

        // Draw canvas to surface
        let surface_texture = self.get_surface_texture()?;
        let surface_view = &surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.upscale_renderer.render(&mut encoder, surface_view);

        // Finish and present surface
        self.ctx.queue.submit(Some(encoder.finish()));
        surface_texture.present();
        Ok(())
    }
}

struct Context {
    device: wgpu::Device,
    queue: wgpu::Queue,
    limits: wgpu::Limits,
    shaders: wgpu::ShaderModule,
    canvas: Canvas,
    quad_buffer: wgpu::Buffer,
}

/// A texture that can both be a target and source
struct Canvas {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    color_format: wgpu::TextureFormat,
    size: Size,
    _dimensions_buffer: wgpu::Buffer,
    dimensions_layout: wgpu::BindGroupLayout,
    dimensions_bind_group: wgpu::BindGroup,
}

impl Canvas {
    fn new(
        device: &wgpu::Device,
        name: &str,
        size: Size,
        color_format: wgpu::TextureFormat,
    ) -> Self {
        let dimensions_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(name),
            contents: bytemuck::cast_slice(&[size.width, size.height, 0, 0]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let dimensions_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(name),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    // minimum size has to be 16 rather than 8 due to WebGL2 layout rules
                    min_binding_size: Some(std::num::NonZeroU64::new(16).unwrap()),
                },
                count: None,
            }],
        });

        let dimensions_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(name),
            layout: &dimensions_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &dimensions_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(name),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: color_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[color_format],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            _texture: texture,
            view,
            color_format,
            size,
            _dimensions_buffer: dimensions_buffer,
            dimensions_layout,
            dimensions_bind_group,
        }
    }
}
