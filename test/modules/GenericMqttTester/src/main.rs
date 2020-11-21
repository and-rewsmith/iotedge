use anyhow::Result;
use futures_util::stream::TryStreamExt;
use tokio;
use tracing::{error, info, subscriber, Level};
use tracing_subscriber::fmt::Subscriber;

use mqtt3::{
    proto::{QoS, SubscribeTo},
    Client,
};

use mqtt_broker_tests_util::client;
use mqtt_util::client_io::ClientIoSource;

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();
    info!("Starting generic mqtt test module");

    let mut client = client::create_client_from_module_env();

    info!("subscribing to dummy topic");
    client
        .subscribe(SubscribeTo {
            topic_filter: "temp/1".to_string(),
            qos: QoS::AtLeastOnce,
        })
        .unwrap();

    info!("polling client");

    poll_client(client).await;

    Ok(())
}

async fn poll_client(mut client: Client<ClientIoSource>) {
    while let Ok(Some(event)) = client.try_next().await {
        info!("received event {:?}", event)
    }

    error!("stopped polling client");
}

fn init_logging() {
    let subscriber = Subscriber::builder().with_max_level(Level::TRACE).finish();
    let _ = subscriber::set_global_default(subscriber);
}
