use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use super::{Assets, Renderer};

impl Assets for () {
    type Error = ();
    type Handle = ();

    fn init() -> Result<Self, Self::Error> {
        Ok(())
    }

    fn get<T>(&self, _: Self::Handle) -> Option<&'static T> {
        None
    }
}

impl Renderer for () {
    type Error = ();
    type Scene = ();

    fn init<C: HasDisplayHandle + HasWindowHandle>(_: &C) -> Result<Self, Self::Error> {
        Ok(())
    }

    fn render(&mut self, _: Self::Scene) -> Result<(), Self::Error> {
        Ok(())
    }

    fn resize(&mut self, _: (u32, u32)) -> Result<(), Self::Error> {
        Ok(())
    }
}
