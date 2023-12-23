use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use super::Context;

struct Winit {
    event_loop: EventLoop<()>,
    window: Window,
}

impl HasWindowHandle for Winit {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        self.window.window_handle()
    }
}

impl HasDisplayHandle for Winit {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        self.window.display_handle()
    }
}

impl Context for Winit {
    type Error = ();

    fn init() -> Result<Self, Self::Error> {
        let event_loop = EventLoop::new().map_err(|_| ())?;
        let mut window_builder = WindowBuilder::new().with_title("TODO");
        window_builder = window_builder.with_resizable(false);
        let window = window_builder.build(&event_loop).map_err(|_| ())?;
        Ok(Self { event_loop, window })
    }

    fn start_event_loop(self) -> Result<(), Self::Error> {
        self.event_loop
            .run(move |event, target| {
                target.set_control_flow(ControlFlow::Poll);
                match event {
                    Event::WindowEvent { window_id, event } if window_id == self.window.id() => {
                        match event {
                            winit::event::WindowEvent::Resized(_) => todo!(),
                            winit::event::WindowEvent::Focused(_) => todo!(),
                            winit::event::WindowEvent::KeyboardInput { .. } => todo!(),
                            winit::event::WindowEvent::ModifiersChanged(_) => todo!(),
                            winit::event::WindowEvent::CursorMoved { .. } => todo!(),
                            winit::event::WindowEvent::MouseWheel { .. } => todo!(),
                            winit::event::WindowEvent::MouseInput { .. } => todo!(),
                            winit::event::WindowEvent::TouchpadMagnify { .. } => todo!(),
                            winit::event::WindowEvent::ScaleFactorChanged { .. } => todo!(),
                            winit::event::WindowEvent::Occluded(_) => todo!(),
                            winit::event::WindowEvent::RedrawRequested => todo!(),
                            winit::event::WindowEvent::Destroyed
                            | winit::event::WindowEvent::CloseRequested => target.exit(),
                            _ => {}
                        }
                    }
                    Event::DeviceEvent { .. } => todo!(),
                    Event::AboutToWait => self.window.request_redraw(),
                    _ => {}
                };
            })
            .map_err(|_| ())
    }
}
