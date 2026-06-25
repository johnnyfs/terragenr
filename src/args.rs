use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long = "log-level", default_value_t = log::Level::Info)]
    pub log_level: log::Level,

    #[arg(short, long = "scene-spec")]
    pub scene_spec_path: String,

    #[arg(short = 'W', long = "width", default_value_t = 1920)]
    pub width: u32,

    #[arg(short = 'H', long = "height", default_value_t = 1080)]
    pub height: u32,

    #[arg(long = "debug-gpu", default_value_t = false)]
    pub debug_gpu: bool,
}

impl Args {
    pub fn parse_() -> Args {
        Args::parse()
    }
}