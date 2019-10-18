use std::env::current_dir;
use std::path::PathBuf;

pub fn init() {
    println!("paths::SHADER_SOURCES: {:?}", SHADER_SOURCES.as_path());
    println!("paths::SHADERS: {:?}", SHADERS.as_path());
    println!("paths::IMAGES: {:?}", IMAGES.as_path());
}

lazy_static! {
    pub static ref SHADER_SOURCES: PathBuf = {
        let mut path = current_dir().unwrap();
        path.push("data");
        path.push("shader_sources");
        path
    };
    pub static ref SHADERS: PathBuf = {
        let mut path = current_dir().unwrap();
        path.push("data");
        path.push("shaders");
        println!("paths::SHADERS: {:?}", path);
        path
    };
    pub static ref IMAGES: PathBuf = {
        let mut path = current_dir().unwrap();
        path.push("data");
        path.push("images");
        println!("paths::IMAGES: {:?}", path);
        path
    };
}
