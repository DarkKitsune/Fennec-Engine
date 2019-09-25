//use colored::Colorize;

pub fn init() {
    /*
    std::panic::set_hook(Box::new(|panic_info| {
        let location = match panic_info.location() {
            Some(location) => format!(
                ", {}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            ),
            None => String::from(""),
        };
        if let Some(message) = panic_info.payload().downcast_ref::<&str>() {
            println!("{}", format!("Panic at \"{}\"{}\nnote: run with `RUST_BACKTRACE=1` environment variable to display a backtrace.", *message, location).red());
            return;
        }
        if let Some(message) = panic_info.payload().downcast_ref::<String>() {
            println!("{}", format!("Panic at \"{}\"{}\nnote: run with `RUST_BACKTRACE=1` environment variable to display a backtrace.", *message, location).red());
            return;
        }
    }))*/
}
