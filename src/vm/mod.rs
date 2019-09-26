pub mod graphicsengine;
pub mod scriptengine;

use crate::error::FennecError;
use crate::fwindow::FWindow;
use glutin::{Event, WindowEvent};
use graphicsengine::GraphicsEngine;
use scriptengine::ScriptEngine;
use std::cell::RefCell;
use std::rc::Rc;

/// A Fennec VM
pub struct VM {
    script_engine: ScriptEngine,
    graphics_engine: GraphicsEngine,
    window: Rc<RefCell<FWindow>>,
}

impl VM {
    /// VM factory method
    pub fn new(window: FWindow) -> Result<Self, FennecError> {
        let window = Rc::new(RefCell::new(window));
        let script_engine = ScriptEngine::new();
        script_engine.register_core_libraries()?;
        let graphics_engine = GraphicsEngine::new(&window)?;
        Ok(Self {
            script_engine,
            graphics_engine,
            window,
        })
    }

    /// Get the script engine
    pub fn script_engine(&self) -> &ScriptEngine {
        &self.script_engine
    }

    /// Get the script engine
    pub fn script_engine_mut(&mut self) -> &mut ScriptEngine {
        &mut self.script_engine
    }

    /// Get the graphics engine
    pub fn graphics_engine(&self) -> &GraphicsEngine {
        &self.graphics_engine
    }

    /// Get the graphics engine
    pub fn graphics_engine_mut(&mut self) -> &mut GraphicsEngine {
        &mut self.graphics_engine
    }

    /// Get the window
    pub fn window(&self) -> &Rc<RefCell<FWindow>> {
        &self.window
    }

    /// Start the VM
    pub fn start(&mut self) -> Result<(), FennecError> {
        let mut running = true;
        while running {
            self.do_events(&mut running)?;
            self.graphics_engine_mut().draw()?;
        }
        self.graphics_engine().stop()?;
        Ok(())
    }

    pub fn do_events(&mut self, running: &mut bool) -> Result<(), FennecError> {
        for ev in self.window().try_borrow_mut()?.poll_events()? {
            if let Event::WindowEvent { event, .. } = ev {
                if let WindowEvent::CloseRequested = event {
                    *running = false;
                }
            }
        }
        Ok(())
    }
}
