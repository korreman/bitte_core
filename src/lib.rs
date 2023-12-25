//! _Very_ generic framework for running a game.
//!
//! This should define the basic framework for how a game is structured.
//! It should perform window handling, keyboard and mouse IO.

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::fmt::Debug;

pub mod unit;
pub mod winit;

pub trait Game: Sized {
    const TITLE: &'static str;

    type Error: Debug;
    /// The asset manager to use.
    type Assets: Assets;
    /// The surrounding context engine (window, web page, etc) to use.
    type Context: Context;
    /// The rendering engine to use.
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
    fn init(title: &str) -> Result<Self, Self::Error>;
    fn start_event_loop<G>(self, runner: GameRunner<G>) -> Result<(), Self::Error>
    where
        G: Game<Context = Self>;
}

/// A graphics renderer.
pub trait Renderer: Sized {
    type Error: Debug;
    type Scene;

    fn init<C: HasDisplayHandle + HasWindowHandle>(context: &C) -> Result<Self, Self::Error>;
    fn resize(&mut self, size: (u32, u32)) -> Result<(), Self::Error>;
    fn render(&mut self, scene: Self::Scene) -> Result<(), Self::Error>;
}

pub struct GameRunner<G: Game> {
    state: G,
    assets: G::Assets,
    renderer: G::Renderer,
}

impl<G: Game> GameRunner<G> {
    pub fn run() {
        let context: G::Context = Context::init(G::TITLE).expect("failed to initialize context");
        let assets: G::Assets = Assets::init().expect("failed to initialize assets");
        let renderer: G::Renderer =
            Renderer::init(&context).expect("failed to initialize renderer");
        let state = G::init(&assets).expect("failed to initialize game state");
        let runner = Self {
            state,
            assets,
            renderer,
        };
        context
            .start_event_loop(runner)
            .expect("failed to start event loop");
    }

    fn update(&mut self) {
        self.state.step(&self.assets);
        let scene = self.state.draw(&self.assets);
        self.renderer.render(scene).expect("failed to render scene");
    }

    fn resize(&mut self, dimensions: (u32, u32)) {
        self.renderer.resize(dimensions).expect("failed to resize");
    }
}
