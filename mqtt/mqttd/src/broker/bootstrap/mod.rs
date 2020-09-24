use mqtt_broker::Broker;
use mqtt_broker::{auth::Authorizer, Sidecar, SidecarError};

#[cfg(feature = "edgehub")]
mod edgehub;

#[cfg(feature = "edgehub")]
pub use edgehub::{broker, config, start_server, start_sidecars};

#[cfg(all(not(feature = "edgehub"), feature = "generic"))]
mod generic;

#[cfg(all(not(feature = "edgehub"), feature = "generic"))]
pub use generic::{broker, config, start_server};


struct Bootstrap<Z> 
where
    Z: Authorizer + Send + 'static,
{
    broker: Broker<Z>,
    sidecars: Vec<Box<dyn Sidecar<Error = SidecarError>>>
}

impl<Z> Bootstrap<Z> 
where
    Z: Authorizer + Send + 'static,
{
    fn new(broker: Broker<Z>, settings: Settings) -> Self {
        Self {
            broker,
            sidecars: Vec::new()
        }
    }

    fn add_sidecar<S>(&mut self, sidecar: S)
    where
        S: Sidecar<Error = SidecarError> + 'static
    {
        self.sidecars.push(Box::new(sidecar));
    }

    async fn run(self, shutdown_signal:F) -> Result {
        // create broker server and run it
        info!("starting server...");
        let server_join_handle =
            tokio::spawn(start_server(settings, broker, shutdown_signal));
        
        // init sidecars
        let initied = future::join_all(self.sidecars.iter().map(|s|s.init())).await
        // if error occurred - stop broker
        broker_handle.join(); // wait for broker server
        // when broker stopped 
        let result = sidecars_shutdown.iter().map(|h|h,shutdown());// handle results
        // wait for sidecars to stop
        let resutl = future::join_all(sidecars_joins.iter().map(|j|j.join)).await; // handle results
    }
}