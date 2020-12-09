use std::io::Error;

use anyhow::Result;
use futures_util::{
    future::{self, Either},
    pin_mut,
};
use signal_hook::{iterator::Signals, SIGINT, SIGTERM};
use tokio::{self};
use tracing::{info, info_span, subscriber, Level};
use tracing_subscriber::fmt::Subscriber;

use generic_mqtt_tester::{settings::Settings, tester::MessageTester};
use tracing_futures::Instrument;

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();
    info!("Starting generic mqtt test module");

    let settings = Settings::new()?;

    let tester = MessageTester::new(settings).await?;
    let tester_shutdown = tester.shutdown_handle();

    let test_fut = tester.run().instrument(info_span!("tester"));
    let test_join_handle = tokio::spawn(test_fut);
    let shutdown_fut = listen_for_shutdown();
    pin_mut!(shutdown_fut);

    match future::select(test_join_handle, shutdown_fut).await {
        Either::Left((test_result, _)) => {
            test_result??;
        }
        Either::Right((shutdown, test_join_handle)) => {
            shutdown?;

            tester_shutdown.shutdown().await?;
            test_join_handle.await??;
        }
    };

    Ok(())
}

fn init_logging() {
    let subscriber = Subscriber::builder().with_max_level(Level::TRACE).finish();
    let _ = subscriber::set_global_default(subscriber);
}

async fn listen_for_shutdown() -> Result<(), Error> {
    let signals = Signals::new(&[SIGTERM, SIGINT])?;
    for signal in signals.forever().filter_map(|signal| match signal {
        SIGTERM => Some("SIGTERM"),
        SIGINT => Some("SIGINT"),
        _ => None,
    }) {
        info!("Received {} signal", signal);
        break;
    }

    Ok(())
}
