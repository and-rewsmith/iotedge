use std::collections::HashMap;

use async_trait::async_trait;
use tracing::info;

use mqtt_broker::Sidecar;

use crate::bridge::{Bridge, BridgeError};
use crate::settings::Settings;

/// Controller that handles the settings and monitors changes, spawns new Bridges and monitors shutdown signal.
#[derive(Default)]
pub struct BridgeController {
    bridges: HashMap<String, Bridge>,
    system_address: String,
    device_id: String,
}

#[async_trait]
impl Sidecar for BridgeController {
    type Error = BridgeError;

    async fn init(&mut self) -> Result<(), Self::Error> {
        info!("starting bridge");
        let settings = Settings::new().map_err(BridgeError::LoadingSettings)?;
        if let Some(upstream) = settings.upstream() {
            let nested_bridge = Bridge::new(
                self.system_address.clone(),
                self.device_id.clone(),
                upstream.clone(),
            );

            self.bridges
                .insert(upstream.name().to_string(), nested_bridge);
        } else {
            info!("No upstream settings detected. Not starting bridge.")
        };
        Ok(())
    }

    async fn run(self) {
        for (_, bridge) in self.bridges {
            bridge.start().await;
        }
    }
}

impl BridgeController {
    pub fn new(system_address: String, device_id: String) -> Self {
        Self {
            bridges: HashMap::new(),
            system_address,
            device_id,
        }
    }
}
