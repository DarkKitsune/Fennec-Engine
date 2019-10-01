use std::env::current_dir;
use std::path::PathBuf;

lazy_static! {
    pub static ref SHADER_SOURCES: PathBuf = {
        let mut path = current_dir().unwrap();
        path.push("data");
        path.push("shader_sources");
        println!("paths::SHADER_SOURCES: {:?}", path);
        path
    };
    pub static ref SHADERS: PathBuf = {
        let mut path = current_dir().unwrap();
        path.push("data");
        path.push("shaders");
        println!("paths::SHADERS: {:?}", path);
        path
    };
}
