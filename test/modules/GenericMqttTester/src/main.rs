use std::{env, time::Duration};

use anyhow::Result;
use futures_util::stream::TryStreamExt;
use tokio;
use tracing::{error, info, subscriber, Level};
use tracing_subscriber::fmt::Subscriber;

use mqtt3::{
    proto::{QoS, SubscribeTo},
    Client,
};

use mqtt_util::client_io::{
    ClientIoSource, CredentialProviderSettings, Credentials, SasTokenSource, TcpConnection,
    TrustBundleSource,
};

// TODO: read from env var
const API_VERSION: &str = "2010-01-01";

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();

    info!("Starting generic mqtt test module");

    let credentials = get_credentials();
    let trust_bundle_source = TrustBundleSource::new(credentials.clone());
    let token_source = SasTokenSource::new(credentials.clone());
    let addr = "edgeHub:8883";
    let tcp_connection = TcpConnection::new(addr, Some(token_source), Some(trust_bundle_source));
    let io_source = ClientIoSource::Tls(tcp_connection);

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
    io_source: ClientIoSource,
) -> Client<ClientIoSource> {
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

    let max_reconnect_backoff = Duration::from_secs(60);
    let keep_alive = Duration::from_secs(60);
    Client::new(
        Some(client_id),
        username,
        None,
        io_source,
        max_reconnect_backoff,
        keep_alive,
    )
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

// TODO: refactor to read from env and return io source, username, password
pub fn get_credentials() -> Credentials {
    info!("getting module env vars...");
    let iothub_hostname = env::var("IOTEDGE_IOTHUBHOSTNAME").unwrap();
    let gateway_hostname = env::var("IOTEDGE_GATEWAYHOSTNAME").unwrap();
    let device_id = env::var("IOTEDGE_DEVICEID").unwrap();
    let module_id = env::var("IOTEDGE_MODULEID").unwrap();
    let generation_id = env::var("IOTEDGE_MODULEGENERATIONID").unwrap();
    let workload_uri = env::var("IOTEDGE_WORKLOADURI").unwrap();

    let credential_provider_settings = CredentialProviderSettings::new(
        iothub_hostname,
        gateway_hostname,
        device_id,
        module_id,
        generation_id,
        workload_uri,
    );

    Credentials::Provider(credential_provider_settings)
}
