use anyhow::Result;
use bytes::Bytes;
use futures_util::stream::TryStreamExt;
use time::Duration;
use tokio::{self, time};
use tracing::{error, info, subscriber, Level};
use tracing_subscriber::fmt::Subscriber;

use mqtt3::{
    proto::{Publication, QoS, SubscribeTo},
    Client, Event, PublishHandle, ReceivedPublication, UpdateSubscriptionHandle,
};
use mqtt_broker_tests_util::client;
use mqtt_util::client_io::ClientIoSource;

use generic_mqtt_tester::settings::{Settings, TestScenario};

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
- shutdown

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

we need some abstraction for running the test
    parse settings
    subscribe
    wait
    send messages + poll client (includes receive messages) + manage expiry
*/

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();
    info!("Starting generic mqtt test module");

    let settings = Settings::default().merge_env()?;

    run_test(settings).await?;

    Ok(())
}

async fn make_test_subscriptions(
    mut subscription_handle: UpdateSubscriptionHandle,
    test_scenario: TestScenario,
) -> Result<()> {
    match test_scenario {
        TestScenario::Receive => {
            subscription_handle
                .subscribe(SubscribeTo {
                    topic_filter: "forwards/1".to_string(),
                    qos: QoS::AtLeastOnce,
                })
                .await?;
        }
        TestScenario::Send => {
            subscription_handle
                .subscribe(SubscribeTo {
                    topic_filter: "backwards/1".to_string(),
                    qos: QoS::AtLeastOnce,
                })
                .await?;
        }
    }

    Ok(())
}

async fn run_test(settings: Settings) -> Result<()> {
    let client = client::create_client_from_module_env();

    // TODO: make these in a way where the subscriptions get made immediately
    info!("making test subscriptions");
    let subscription_handle = client.update_subscription_handle()?;
    let test_scenario = settings.test_scenario();
    make_test_subscriptions(subscription_handle, test_scenario.clone()).await?;

    info!("waiting for test start delay");
    time::delay_for(Duration::from_secs(15)).await;

    info!("starting client poll task");
    let publish_handle = client.publish_handle().unwrap();
    let client_poll_join_handle = tokio::spawn(poll_client(
        client,
        publish_handle.clone(),
        test_scenario.clone(),
    ));

    if let TestScenario::Send = settings.test_scenario() {
        info!("starting message publish task");
        let publish_join_handle = tokio::spawn(publish_forward_messages(publish_handle));
    }

    Ok(())
}

// TODO: maybe pass some sort of message handler abstraction
async fn poll_client(
    mut client: Client<ClientIoSource>,
    publish_handle: PublishHandle,
    test_scenario: TestScenario,
) {
    while let Ok(Some(event)) = client.try_next().await {
        info!("received event {:?}", event);

        match event {
            Event::NewConnection { .. } => {
                info!("received new connection");
            }
            Event::Publication(publication) => {
                info!("received publication");
                handle_publication(publication, publish_handle.clone(), test_scenario.clone())
                    .await;
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

// TODO: refactor to some handler trait
// TODO: add const for forwards / backwards topics
// TODO: analyzer client
async fn handle_publication(
    received_publication: ReceivedPublication,
    mut publish_handle: PublishHandle,
    test_scenario: TestScenario,
) {
    match test_scenario {
        TestScenario::Receive => {
            info!(
                "sending received publication {:?} back to downstream broker",
                received_publication.payload
            );

            let publication = Publication {
                topic_name: "backwards".to_string(),
                qos: QoS::ExactlyOnce,
                retain: true,
                payload: received_publication.payload,
            };
            publish_handle.publish(publication).await.unwrap();
        }
        TestScenario::Send => info!(
            "reporting result for received publication {:?}",
            received_publication.payload
        ),
    }
}

async fn publish_forward_messages(mut publish_handle: PublishHandle) {
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

fn init_logging() {
    let subscriber = Subscriber::builder().with_max_level(Level::TRACE).finish();
    let _ = subscriber::set_global_default(subscriber);
}
