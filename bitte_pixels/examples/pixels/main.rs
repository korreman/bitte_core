use graphics::Size;

use pollster::block_on;
use rand::prelude::*;
use winit::dpi::PhysicalSize;

const WIDTH: f32 = 240f32;
const MAX_VEL: f32 = 2f32;
const PIXEL_COUNT: usize = 1000;

struct Pixel {
    x: f32,
    y: f32,
    r: f32,
    g: f32,
    b: f32,
    a: f32,

    vx: f32,
    vy: f32,
}

impl Pixel {
    fn to_renderpixel(&self) -> graphics::PrimitiveVertex {
        graphics::PrimitiveVertex {
            position: [self.x, self.y],
            color: [self.r, self.g, self.b, self.a],
        }
    }

    fn step(&mut self) {
        self.x = (self.x + self.vx) % WIDTH;
        self.y = (self.y + self.vy) % WIDTH;
        if self.x < 0f32 {
            self.x += WIDTH;
        }
        if self.y < 0f32 {
            self.y += WIDTH;
        }
    }

    fn make_many(count: usize) -> Vec<Pixel> {
        let mut rng = thread_rng();
        let mut pixels = Vec::with_capacity(count);
        for _ in 0..count {
            let x: f32 = rng.gen_range(0f32..WIDTH);
            let y: f32 = rng.gen_range(0f32..WIDTH);
            let vx: f32 = rng.gen_range(-MAX_VEL..MAX_VEL);
            let vy: f32 = rng.gen_range(-MAX_VEL..MAX_VEL);

            let r: f32 = rng.gen_range(0.0..1.0);
            let g: f32 = rng.gen_range(0.0..1.0);
            let b: f32 = rng.gen_range(0.0..1.0);
            let a: f32 = rng.gen_range(0.0..1.0);

            let pixel = Pixel {
                x,
                y,
                r,
                g,
                b,
                a,
                vx,
                vy,
            };
            pixels.push(pixel);
        }
        pixels
    }
}

fn main() {
    run()
}

fn run() {
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    println!("Created event loop");
    let window = winit::window::WindowBuilder::new()
        .with_title("Pixels example!")
        .with_inner_size(PhysicalSize::new(WIDTH * 3f32, WIDTH * 3f32))
        .with_resizable(false)
        .build(&event_loop)
        .expect("window building");

    println!("Created window");
    let window_size = Size {
        width: window.inner_size().width,
        height: window.inner_size().height,
    };

    let mut renderer = block_on(graphics::Renderer::new(
        &window,
        window_size,
        Size {
            width: WIDTH as u32,
            height: WIDTH as u32,
        },
    ))
    .unwrap();
    println!("Created renderer");

    let sprite_sheet = renderer.create_sprite_sheet_builder("").build();

    let mut pixels = Pixel::make_many(PIXEL_COUNT);

    let _ = event_loop.run(|event, target| {
        if let winit::event::Event::WindowEvent { event, .. } = event {
            match event {
                winit::event::WindowEvent::Resized(size) => renderer.resize_surface(Size {
                    width: size.width,
                    height: size.height,
                }),
                winit::event::WindowEvent::CloseRequested => {
                    target.exit();
                }
                winit::event::WindowEvent::RedrawRequested => {
                    let renderpixels = pixels
                        .iter_mut()
                        .map(|p| {
                            p.step();
                            p.to_renderpixel()
                        })
                        .collect();

                    let scene = graphics::Scene {
                        rectangles: Vec::new(),
                        sprites: Vec::new(),
                        pixels: renderpixels,
                        linestrips: Vec::new(),
                        circles: vec![graphics::Circle {
                            offset: [24, 24],
                            diameter: 13,
                            color: [1.0, 1.0, 1.0, 1.0],
                        }],
                    };
                    renderer
                        .render(&sprite_sheet, &scene)
                        .expect("draw to screen");
                    window.request_redraw();
                }
                _ => (),
            }
        }
    });
}
