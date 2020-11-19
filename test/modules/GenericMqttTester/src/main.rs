use std::time::Duration;

use anyhow::Result;
use futures_util::stream::TryStreamExt;
use tokio;
use tracing::{error, info, subscriber, Level};
use tracing_subscriber::fmt::Subscriber;

use mqtt3::{
    proto::{QoS, SubscribeTo},
    Client,
};

use generic_mqtt_tester::{
    io::{BridgeIoSource, TcpConnection},
    token_source::{self, Credentials, SasTokenSource, TrustBundleSource},
};
use token_source::CredentialProviderSettings;

// TODO: read from env var
const API_VERSION: &str = "2010-01-01";

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();

    info!("Starting generic mqtt test module");

    let credentials = token_source::get_credentials();
    let trust_bundle_source = TrustBundleSource::new(credentials.clone());
    let token_source = SasTokenSource::new(credentials.clone());
    let addr = "1882";
    let tcp_connection = TcpConnection::new(addr, Some(token_source), Some(trust_bundle_source));
    let io_source = BridgeIoSource::Tls(tcp_connection);

    // make client using io source
    if let Credentials::Provider(provider_settings) = credentials {
        let mut client = create_client(provider_settings, io_source).await;

        info!("subscribing to dummy topic");

        client
            .subscribe(SubscribeTo {
                topic_filter: "temp/1".to_string(),
                qos: QoS::AtLeastOnce,
            })
            .unwrap();

        info!("polling client");

        poll_client(client).await;
    } else {
        error!("wrong credentials format");
    }

    Ok(())
}

async fn create_client(
    provider_settings: CredentialProviderSettings,
    io_source: BridgeIoSource,
) -> Client<BridgeIoSource> {
    let max_reconnect_backoff = Duration::from_secs(60);
    let keep_alive = Duration::from_secs(60);

    let client_id = format!(
        "{}/{}",
        provider_settings.device_id().to_owned(),
        provider_settings.module_id().to_owned()
    );
    //TODO: handle properties that are sent by client in username (modelId, authchain)
    let username = Some(format!(
        "{}/{}/{}/?api-version={}",
        provider_settings.iothub_hostname().to_owned(),
        provider_settings.device_id().to_owned(),
        provider_settings.module_id().to_owned(),
        API_VERSION.to_owned()
    ));

    info!(
        "creating client with client id ({:?}) and username ({:?})",
        client_id, username,
    );

    Client::new(
        Some(client_id),
        username,
        None,
        io_source,
        max_reconnect_backoff,
        keep_alive,
    )
}

async fn poll_client(mut client: Client<BridgeIoSource>) {
    while let Ok(Some(event)) = client.try_next().await {
        info!("received event {:?}", event)
    }

    error!("stopped polling client");
}

fn init_logging() {
    let subscriber = Subscriber::builder().with_max_level(Level::TRACE).finish();
    let _ = subscriber::set_global_default(subscriber);
}
