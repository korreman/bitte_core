//! _Very_ generic framework for running a game.
//!
//! This should define the basic framework for how a game is structured.
//! It should perform window handling, keyboard and mouse IO.

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::fmt::Debug;

mod winit;

pub trait Game: Sized {
    type Error: Debug;
    type Assets: Assets;
    type Context: Context;
    type Renderer: Renderer;

    /// Instantiate the game state.
    fn init(assets: &Self::Assets) -> Result<Self, Self::Error>;

    /// Perform a world step.
    fn step(&mut self, assets: &Self::Assets);

    /// Generate a scene from the game state,
    /// that can be rendered by the [Renderer].
    fn draw(&self, assets: &Self::Assets) -> <Self::Renderer as Renderer>::Scene;
}

/// A map of static game assets.
pub trait Assets: Sized {
    type Error: Debug;
    type Handle: Copy;

    fn init() -> Result<Self, Self::Error>;
    fn get<T>(&self, handle: Self::Handle) -> Option<&'static T>;
}

/// Window context and event handler.
pub trait Context: Sized + HasDisplayHandle + HasWindowHandle {
    type Error: Debug;
    fn init() -> Result<Self, Self::Error>;
    fn start_event_loop(self) -> Result<(), Self::Error>;
}

/// A graphics renderer.
pub trait Renderer: Sized {
    type Error: Debug;
    type Scene;

    fn init<C: HasDisplayHandle + HasWindowHandle>(context: &C) -> Result<Self, Self::Error>;
    fn render(&mut self, scene: Self::Scene) -> Result<(), Self::Error>;
}

trait Private {}
impl<T: Game> Private for T {}
impl<T: Game + Private> RunGame for T {}

/// Sealed function for running the game.
#[allow(private_bounds)]
pub trait RunGame: Game + Private {
    fn run() {
        let context: Self::Context = Context::init().expect("failed to initialize context");
        let _assets: Self::Assets = Assets::init().expect("failed to initialize assets");
        let _renderer: Self::Renderer =
            Renderer::init(&context).expect("failed to initialize renderer");
        context.start_event_loop().expect("failed to start event loop");
    }
}
