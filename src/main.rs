use log;
use stderrlog;

mod args;
mod app;
mod scene;
mod video;

fn main() {
    let args = args::Args::parse_();

    stderrlog::new()
        .verbosity(args.log_level)
        .init().unwrap();

    let app = match app::App::from(&args) {
        Ok(value) => value,
        Err(message) => { 
            log::error!("Initialization failed: {}", message);
            std::process::exit(1)
        }
    };

    match app.run() {
        Ok(_) => std::process::exit(0),
        Err(message) => {
            log::error!("Execution failed: {}", message);
            std::process::exit(1)
        }
    }
}
