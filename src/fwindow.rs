use crate::error::FennecError;
use glutin::{EventsLoop, Window, WindowBuilder};

/// A Fennec window
pub struct FWindow {
    event_loop: EventsLoop,
    window: Window,
}

impl FWindow {
    /// FWindow factory method
    pub fn new() -> Result<Self, FennecError> {
        let event_loop = EventsLoop::new();
        let window_builder = WindowBuilder::new().with_title("Aaaa");
        let window = window_builder.build(&event_loop)?;
        Ok(FWindow { event_loop, window })
    }

    /// Get the event loop
    pub fn event_loop(&self) -> &EventsLoop {
        &self.event_loop
    }

    /// Get the event loop
    pub fn event_loop_mut(&mut self) -> &mut EventsLoop {
        &mut self.event_loop
    }

    /// Get the glutin window
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Get the glutin window
    pub fn window_mut(&mut self) -> &mut Window {
        &mut self.window
    }

    /// Get the client size (inner size) of the window in points
    pub fn client_size_points(&self) -> Result<(u32, u32), FennecError> {
        let client_size = self
            .window()
            .get_inner_size()
            .ok_or_else(|| FennecError::new("Window does not exist"))?;
        Ok((client_size.width as u32, client_size.height as u32))
    }

    /// Get the client size (inner size) of the window in pixels
    pub fn client_size_pixels(&self) -> Result<(u32, u32), FennecError> {
        let hidpi_factor = self.window().get_hidpi_factor();
        let client_size = self
            .window()
            .get_inner_size()
            .ok_or_else(|| FennecError::new("Window does not exist"))?;
        Ok((
            (client_size.width * hidpi_factor) as u32,
            (client_size.height * hidpi_factor) as u32,
        ))
    }
}
