use fmt::Display;
use std::error::*;
use std::fmt;

/// An error in the Fennec VM
#[derive(Debug)]
pub struct FennecError {
    description: String,
    cause: Option<Box<dyn Error>>,
}

impl FennecError {
    /// FennecError factory method
    pub fn new<S>(description: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            description: description.into(),
            cause: None,
        }
    }

    /// Factory method for script engine errors
    pub fn script(error: rlua::Error) -> Self {
        FennecError::from_error("Script error occurred", Box::new(error))
    }

    /// Factory method for errors wrapping non-Fennec errors
    pub fn from_error<S>(description: S, cause: Box<dyn Error>) -> Self
    where
        S: Into<String>,
    {
        Self {
            description: description.into(),
            cause: Some(cause),
        }
    }

    /// Get the cause, if there is one
    fn cause(&self) -> &Option<Box<dyn Error>> {
        &self.cause
    }
}

impl Display for FennecError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let possible_cause = self.cause();
        match possible_cause {
            Some(cause) => {
                let cause_desc = self.description();
                if !cause_desc.is_empty() {
                    write!(f, "{}: {}", self.description(), cause)?;
                } else {
                    write!(f, "{}", self.description())?;
                }
            }
            None => {
                write!(f, "{}", self.description())?;
            }
        }

        Ok(())
    }
}

impl Error for FennecError {
    fn description(&self) -> &str {
        &self.description[..]
    }

    fn source(&self) -> Option<&'static dyn Error> {
        None
    }
}

impl From<&str> for FennecError {
    fn from(error: &str) -> FennecError {
        FennecError::new(error)
    }
}

impl From<rlua::Error> for FennecError {
    fn from(error: rlua::Error) -> FennecError {
        FennecError::script(error)
    }
}

impl From<glutin::WindowCreationError> for FennecError {
    fn from(error: glutin::WindowCreationError) -> FennecError {
        FennecError::from_error("Window creation error occurred", Box::new(error))
    }
}

impl From<ash::LoadingError> for FennecError {
    fn from(error: ash::LoadingError) -> FennecError {
        FennecError::from_error("Ash error occurred", Box::new(error))
    }
}

impl From<ash::InstanceError> for FennecError {
    fn from(error: ash::InstanceError) -> FennecError {
        FennecError::from_error("Ash error occurred", Box::new(error))
    }
}

impl From<ash::vk::Result> for FennecError {
    fn from(error: ash::vk::Result) -> FennecError {
        FennecError::from_error("Vulkan error occurred", Box::new(error))
    }
}

impl From<std::cell::BorrowError> for FennecError {
    fn from(error: std::cell::BorrowError) -> FennecError {
        FennecError::from_error("Could not borrow from cell", Box::new(error))
    }
}

impl From<std::cell::BorrowMutError> for FennecError {
    fn from(error: std::cell::BorrowMutError) -> FennecError {
        FennecError::from_error("Could not borrow mutibly from cell", Box::new(error))
    }
}

impl From<std::ffi::NulError> for FennecError {
    fn from(error: std::ffi::NulError) -> FennecError {
        FennecError::from_error("Could not create CString", Box::new(error))
    }
}

impl From<std::io::Error> for FennecError {
    fn from(error: std::io::Error) -> FennecError {
        FennecError::from_error("IO error occurred", Box::new(error))
    }
}

impl From<std::string::FromUtf8Error> for FennecError {
    fn from(error: std::string::FromUtf8Error) -> FennecError {
        FennecError::from_error("Could not convert string from UTF-8", Box::new(error))
    }
}
