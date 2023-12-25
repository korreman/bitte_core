use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use super::{Context, Game, GameRunner};

pub struct Winit {
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

    fn init(title: &str) -> Result<Self, Self::Error> {
        let event_loop = EventLoop::new().map_err(|_| ())?;
        let mut window_builder = WindowBuilder::new().with_title(title);
        window_builder = window_builder.with_resizable(false);
        let window = window_builder.build(&event_loop).map_err(|_| ())?;
        Ok(Self { event_loop, window })
    }

    fn start_event_loop<G>(self, mut runner: GameRunner<G>) -> Result<(), Self::Error>
    where
        G: Game<Context = Self>,
    {
        self.event_loop
            .run(move |event, target| {
                target.set_control_flow(ControlFlow::Poll);
                match event {
                    Event::WindowEvent { window_id, event } if window_id == self.window.id() => {
                        match event {
                            WindowEvent::RedrawRequested => runner.update(),
                            WindowEvent::Resized(size) => runner.resize((size.width, size.height)),
                            WindowEvent::Focused(_) => todo!(),

                            WindowEvent::KeyboardInput { .. } => todo!(),
                            WindowEvent::ModifiersChanged(_) => todo!(),

                            WindowEvent::CursorMoved { .. } => todo!(),
                            WindowEvent::MouseWheel { .. } => todo!(),
                            WindowEvent::MouseInput { .. } => todo!(),
                            WindowEvent::TouchpadMagnify { .. } => todo!(),

                            WindowEvent::Occluded(_) => todo!(),

                            WindowEvent::Destroyed | WindowEvent::CloseRequested => target.exit(),
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
