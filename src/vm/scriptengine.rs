use crate::error::FennecError;
use rlua::Lua;

/// A Fennec script engine
#[derive(Default)]
pub struct ScriptEngine {
    lua: Lua,
}

impl ScriptEngine {
    /// ScriptEngine factory method
    pub fn new() -> Self {
        let lua = Lua::new();
        Self { lua }
    }

    /// Register the core libraries
    pub fn register_core_libraries(&self) -> Result<(), FennecError> {
        self.lua.context(|context| {
            let globals = context.globals();
            // fennec library
            {
                let fennec = context.create_table()?;
                // fennec.version()
                fennec.set(
                    "version",
                    context.create_function(|_, ()| {
                        Ok(format!(
                            "{}.{}.{}",
                            crate::manifest::ENGINE_VERSION.0,
                            crate::manifest::ENGINE_VERSION.1,
                            crate::manifest::ENGINE_VERSION.2
                        ))
                    })?,
                )?;
                globals.set("fennec", fennec)?;
            }
            // Done
            Ok(())
        })
    }
}
