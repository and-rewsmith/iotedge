use anyhow::Result;
use tokio;
use tracing::{info, subscriber, Level};
use tracing_subscriber::fmt::Subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();

    info!("Starting generic mqtt test module");

    Ok(())
}

fn init_logging() {
    let subscriber = Subscriber::builder().with_max_level(Level::TRACE).finish();
    let _ = subscriber::set_global_default(subscriber);
}
