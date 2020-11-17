use std::{env, time::Duration};

use anyhow::Result;
use async_trait::async_trait;
use tokio;
use tracing::{info, subscriber, Level};
use tracing_subscriber::fmt::Subscriber;

use mqtt3::Event;
use mqtt_bridge::{
    client::{Handled, MqttClient, MqttClientConfig, MqttEventHandler},
    settings::CredentialProviderSettings,
    settings::Credentials,
};

struct EventHandler {}

#[async_trait]
impl MqttEventHandler for EventHandler {
    type Error = EventHandlerError;

    async fn handle(&mut self, _event: Event) -> Result<Handled, Self::Error> {
        Ok(Handled::Fully)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EventHandlerError {}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();

    info!("Starting generic mqtt test module");

    let addr = "1882";
    let keep_alive = Duration::from_secs(60);
    let clean_session = true;
    let credentials = get_credentials();

    let config = MqttClientConfig::new(addr, keep_alive, clean_session, credentials);
    let event_handler = EventHandler {};

    let mut client = MqttClient::tls(config, event_handler).unwrap();
    info!("Created mqtt client");

    client.run().await.unwrap();

    Ok(())
}

fn get_credentials() -> Credentials {
    let iothub_hostname = env::var("iotedge_iothubhostname").unwrap();
    let gateway_hostname = env::var("iotedge_gatewayhostname").unwrap();
    let device_id = env::var("iotedge_deviceid").unwrap();
    let module_id = env::var("iotedge_moduleid").unwrap();
    let generation_id = env::var("generation_id").unwrap();
    let workload_uri = env::var("workload_uri").unwrap();

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

fn init_logging() {
    let subscriber = Subscriber::builder().with_max_level(Level::TRACE).finish();
    let _ = subscriber::set_global_default(subscriber);
}
