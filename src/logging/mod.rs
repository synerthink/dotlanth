use env_logger::Env;
use log::info;

pub fn setup_logging() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    info!("Logging initialized.");
}
