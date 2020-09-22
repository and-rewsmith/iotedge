use std::collections::HashMap;

use tracing::info;

use crate::bridge::{Bridge, BridgeError};
use crate::settings::Settings;

/// Controller that handles the settings and monitors changes, spawns new Bridges and monitors shutdown signal.
#[derive(Default)]
pub struct BridgeController {
    bridges: HashMap<String, Bridge>,
    system_address: String,
    device_id: String,
}

impl BridgeController {
    pub fn new(system_address: String, device_id: String) -> Self {
        Self {
            bridges: HashMap::new(),
            system_address,
            device_id,
        }
    }

    pub async fn start(&mut self) -> Result<(), BridgeError> {
        info!("starting bridge");
        let settings = Settings::new().map_err(BridgeError::LoadingSettings)?;
        if let Some(upstream) = settings.upstream() {
            let nested_bridge = Bridge::new(
                self.system_address.clone(),
                self.device_id.clone(),
                upstream.clone(),
            );

            nested_bridge.start().await?;

            self.bridges
                .insert(upstream.name().to_string(), nested_bridge);
        } else {
            info!("No upstream settings detected. Not starting bridge.")
        };
        Ok(())
    }
}
