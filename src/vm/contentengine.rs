use crate::error::FennecError;
use crate::paths;
use std::fs::File;
use std::path::{Path, PathBuf};

/// The content engine for a VM; handles content loading and caching
pub struct ContentEngine {}

impl ContentEngine {
    /// Gets the root directory for a given type of content
    pub fn content_root(content_type: ContentType) -> &'static Path {
        match content_type {
            ContentType::ShaderModule => &paths::SHADERS,
            ContentType::Image => &paths::IMAGES,
        }
    }

    /// Gets the path to a given content item
    pub fn content_path(name: &str, content_type: ContentType) -> PathBuf {
        let name = format!("{}.{}", name, Self::content_extension(content_type));
        Self::content_root(content_type).join(name)
    }

    /// Gets the file extension for a given type of content
    pub fn content_extension(content_type: ContentType) -> &'static str {
        match content_type {
            ContentType::ShaderModule => "spv",
            ContentType::Image => "png",
        }
    }

    /// Opens a content file for reading
    pub fn open(name: &str, content_type: ContentType) -> Result<File, FennecError> {
        Ok(File::open(Self::content_path(name, content_type))?)
    }
}

/// A type of content
#[derive(Copy, Clone, Debug, PartialEq, Hash)]
pub enum ContentType {
    ShaderModule,
    Image,
}
