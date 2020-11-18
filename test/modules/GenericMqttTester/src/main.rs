use std::time::Duration;

use anyhow::Result;
use tokio;
use tracing::{info, subscriber, Level};
use tracing_subscriber::fmt::Subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();

    info!("Starting generic mqtt test module");

    let addr = "1882";
    let keep_alive = Duration::from_secs(60);
    let clean_session = true;

    Ok(())
}

fn init_logging() {
    let subscriber = Subscriber::builder().with_max_level(Level::TRACE).finish();
    let _ = subscriber::set_global_default(subscriber);
}
