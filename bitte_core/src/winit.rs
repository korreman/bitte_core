use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
    window::{Window, WindowBuilder},
};

use super::{Context, Game, GameRunner};

pub struct Winit {
    event_loop: EventLoop<()>,
    events: Vec<WinitInput>,
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
    type InputEvent = WinitInput;

    fn init(title: &str) -> Result<Self, Self::Error> {
        let event_loop = EventLoop::new().map_err(|_| ())?;
        let mut window_builder = WindowBuilder::new().with_title(title);
        window_builder = window_builder.with_resizable(false);
        let window = window_builder.build(&event_loop).map_err(|_| ())?;
        Ok(Self {
            event_loop,
            events: Vec::new(),
            window,
        })
    }

    fn start_event_loop<G>(mut self, mut runner: GameRunner<G>) -> Result<(), Self::Error>
    where
        G: Game<Context = Self>,
    {
        let event_handler = move |event, target: &EventLoopWindowTarget<()>| {
            target.set_control_flow(ControlFlow::Poll);
            match event {
                Event::WindowEvent { window_id, event } if window_id == self.window.id() => {
                    match event {
                        WindowEvent::RedrawRequested => runner.update(self.events.drain(..)),
                        WindowEvent::Resized(size) => runner.resize((size.width, size.height)),
                        WindowEvent::Focused(x) => self.events.push(WinitInput::Focus(x)),
                        WindowEvent::KeyboardInput { event, .. } => {
                            self.events.push(WinitInput::Keyboard(event))
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            self.events.push(WinitInput::CursorMoved(position))
                        }
                        WindowEvent::MouseInput { state, button, .. } => {
                            self.events.push(WinitInput::MouseInput(state, button))
                        }
                        WindowEvent::Destroyed | WindowEvent::CloseRequested => target.exit(),
                        _ => {}
                    }
                }
                Event::DeviceEvent { .. } => todo!(),
                Event::AboutToWait => self.window.request_redraw(),
                _ => {}
            };
        };
        self.event_loop.run(event_handler).map_err(|_| ())
    }
}

pub enum WinitInput {
    Focus(bool),
    Keyboard(KeyEvent),
    CursorMoved(PhysicalPosition<f64>),
    MouseInput(ElementState, MouseButton),
}
