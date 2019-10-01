extern crate rlua;
extern crate version;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate ash;
extern crate glutin;

#[macro_use]
pub mod error;
pub mod fwindow;
pub mod iteratorext;
pub mod log;
pub mod paths;
pub mod vm;

use fwindow::FWindow;
use vm::VM;

/// The application manifest
pub mod manifest {
    pub const ENGINE_NAME: &str = "Fennec";
    lazy_static! {
        pub static ref ENGINE_VERSION: (u32, u32, u32) = {
            let mut nums = version::version!().split('.').map(|num| {
                num.parse::<u32>()
                    .expect("Version was not in the proper format")
            });
            (
                nums.next().expect("Version was not in the proper format"),
                nums.next().expect("Version was not in the proper format"),
                nums.next().expect("Version was not in the proper format"),
            )
        };
    }
}

/// Entry point
fn main() {
    // Print info
    println!(
        "Fennec {}.{}.{}",
        manifest::ENGINE_VERSION.0,
        manifest::ENGINE_VERSION.1,
        manifest::ENGINE_VERSION.2
    );
    // Initialization
    //log::init();
    // Create Fennec window
    let window = FWindow::new().expect("Could not create window");
    // Create Fennec VM
    let mut vm = VM::new(window).expect("Could not create VM");
    // Start the VM
    vm.start().unwrap();
}
