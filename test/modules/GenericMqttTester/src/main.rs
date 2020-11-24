use anyhow::Result;
use bytes::Bytes;
use futures_util::stream::TryStreamExt;
use time::Duration;
use tokio::{self, time};
use tracing::{error, info, subscriber, Level};
use tracing_subscriber::fmt::Subscriber;

use mqtt3::{
    proto::{Publication, QoS, SubscribeTo},
    Client, Event, PublishHandle,
};

use mqtt_broker_tests_util::client;
use mqtt_util::client_io::ClientIoSource;

// TODO;
/*

sequence:
- startup
- subscribe to topic
- test delay
- test logic
  - send
  - receive and verify order
  - determine if response took too long
- report to TRC if
  - took too long
  - order wrong

settings class
 - test start delay
 - expiry time
 - validation time
 - receive mode / send mode
 - send frequency

if we are sending then:
- spawn task to send that stores messages in state and waits for expiry
- how to determine expiry?

if we are receiving:
- when receive a publication then send one back to broker
*/

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();
    info!("Starting generic mqtt test module");

    test_send().await.unwrap();

    Ok(())
}

async fn test_send() -> Result<()> {
    let mut client = client::create_client_from_module_env();

    info!("subscribing to test topic");
    client
        .subscribe(SubscribeTo {
            topic_filter: "backwards/1".to_string(),
            qos: QoS::AtLeastOnce,
        })
        .unwrap();

    info!("waiting for test start delay");
    time::delay_for(Duration::from_secs(15)).await;

    // TODO: handle shutdown
    info!("starting message publish task");
    let publish_handle = client.publish_handle().unwrap();
    let publish_join_handle = tokio::spawn(publish_messages(publish_handle));

    info!("starting client poll task");
    let client_poll_join_handle = tokio::spawn(poll_client(client));

    Ok(())
}

async fn publish_messages(mut publish_handle: PublishHandle) {
    let mut seq_num = 0;
    loop {
        info!("publishing message to upstream broker");
        let publication = Publication {
            topic_name: "forwards".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::from(seq_num.to_string()),
        };
        publish_handle.publish(publication).await.unwrap();

        info!("waiting for message send delay");
        time::delay_for(Duration::from_secs(1)).await;

        seq_num += 1;
    }
}

async fn poll_client(mut client: Client<ClientIoSource>) {
    while let Ok(Some(event)) = client.try_next().await {
        info!("received event {:?}", event);

        match event {
            Event::NewConnection { .. } => {
                info!("received new connection");
            }
            Event::Publication(publication) => {
                info!("received publication");
            }
            Event::SubscriptionUpdates(_) => {
                info!("received subscription update");
            }
            Event::Disconnected(_) => {
                info!("received disconnect");
            }
        };
    }

    error!("stopped polling client");
}

fn init_logging() {
    let subscriber = Subscriber::builder().with_max_level(Level::TRACE).finish();
    let _ = subscriber::set_global_default(subscriber);
}
