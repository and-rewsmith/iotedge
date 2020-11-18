use std::time::Duration;

use anyhow::Result;
use tokio;
use tracing::{error, info, subscriber, Level};
use tracing_subscriber::fmt::Subscriber;

use mqtt3::Client;

use generic_mqtt_tester::{
    io::{BridgeIoSource, TcpConnection},
    token_source::{self, Credentials, SasTokenSource, TrustBundleSource},
};

// TODO: read from env var
const API_VERSION: &str = "2010-01-01";

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();

    info!("Starting generic mqtt test module");

    let credentials = token_source::get_credentials();

    // get trust bundle
    let trust_bundle_source = TrustBundleSource::new(credentials.clone());

    // get token source
    let token_source = SasTokenSource::new(credentials.clone());

    // make tcp connection using above
    let addr = "1882";
    let tcp_connection = TcpConnection::new(addr, Some(token_source), Some(trust_bundle_source));

    // make io_source using tcp connection
    let io_source = BridgeIoSource::Tls(tcp_connection);

    // make client using io source
    let max_reconnect_backoff = Duration::from_secs(60);
    let keep_alive = Duration::from_secs(60);
    if let Credentials::Provider(provider_settings) = credentials {
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

        let client = Client::new(
            Some(client_id),
            username,
            None,
            io_source,
            max_reconnect_backoff,
            keep_alive,
        );
    } else {
        error!("wrong credentials format");
    }

    Ok(())
}

fn init_logging() {
    let subscriber = Subscriber::builder().with_max_level(Level::TRACE).finish();
    let _ = subscriber::set_global_default(subscriber);
}
