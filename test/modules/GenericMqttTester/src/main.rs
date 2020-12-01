use anyhow::Result;
use bytes::Bytes;
use futures_util::stream::StreamExt;
use futures_util::stream::TryStreamExt;
use mpsc::{UnboundedReceiver, UnboundedSender};
use time::Duration;
use tokio::{self, sync::mpsc, time};
use tracing::{info, subscriber, Level};
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


*************************************************
settings class
 - test start delay
 - expiry time
 - validation time
 - receive mode / send mode
 - send frequency


main
    parse settings
    make test fixture
    run test
    wait for shutdown

test fixture new
    save settings
    make client
    make handler depending on settings
    make unbounded channel
    make shutdown

    wait for subscriptions

test fixture run (consume self)
    start polling client and pass in handler

    parse settings to determine appropriate action
    if send:
        start thread to send messages

*************************************************
*/

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();
    info!("Starting generic mqtt test module");

    let settings = Settings::new()?;

    let client = client::create_client_from_module_env();

    // TODO: make these in a way where the subscriptions get made immediately
    info!("waiting for test start delay");
    time::delay_for(Duration::from_secs(15)).await;

    let publish_handle = client.publish_handle()?;
    let subscription_handle = client.update_subscription_handle()?;
    let test_scenario = settings.test_scenario();

    info!("starting client poll task");
    let (sender, receiver) = mpsc::unbounded_channel::<ReceivedPublication>();
    let client_poll_join_handle = tokio::spawn(poll_client(client, sender, test_scenario.clone()));

    let pub_handler_handle = tokio::spawn(handle_publication(
        receiver,
        publish_handle.clone(),
        test_scenario.clone(),
    ));

    info!("making test subscriptions");
    make_test_subscriptions(subscription_handle, test_scenario.clone()).await?;

    if let TestScenario::Initiate = settings.test_scenario() {
        info!("starting message publish task");
        let publish_join_handle = tokio::spawn(publish_forward_messages(publish_handle));
    }

    client_poll_join_handle.await??;

    Ok(())
}

async fn make_test_subscriptions(
    mut subscription_handle: UpdateSubscriptionHandle,
    test_scenario: TestScenario,
) -> Result<()> {
    match test_scenario {
        TestScenario::Relay => {
            subscription_handle
                .subscribe(SubscribeTo {
                    topic_filter: "forwards/1".to_string(),
                    qos: QoS::AtLeastOnce,
                })
                .await?;
        }
        TestScenario::Initiate => {
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

// TODO: maybe pass some sort of message handler abstraction
async fn poll_client(
    mut client: Client<ClientIoSource>,
    pub_send_channel: UnboundedSender<ReceivedPublication>,
    test_scenario: TestScenario,
) -> Result<()> {
    while let Ok(Some(event)) = client.try_next().await {
        info!("received event {:?}", event);

        match event {
            Event::NewConnection { .. } => {
                info!("received new connection");
            }
            Event::Publication(publication) => {
                info!("received publication");
                pub_send_channel.send(publication)?;
            }
            Event::SubscriptionUpdates(_) => {
                info!("received subscription update");
            }
            Event::Disconnected(_) => {
                info!("received disconnect");
            }
        };
    }

    // TODO: this is probably an error if we stop polling client
    info!("stopped polling client");
    Ok(())
}

// TODO: refactor to some handler trait
// TODO: add const for forwards / backwards topics
// TODO: analyzer client
async fn handle_publication(
    mut receiver: UnboundedReceiver<ReceivedPublication>,
    mut publish_handle: PublishHandle,
    test_scenario: TestScenario,
) -> Result<()> {
    loop {
        let received_publication = receiver.next().await.expect("publication received");

        match test_scenario {
            TestScenario::Relay => {
                info!(
                    "sending received publication {:?} back to downstream broker",
                    received_publication.payload
                );

                let publication = Publication {
                    topic_name: "backwards/1".to_string(),
                    qos: QoS::ExactlyOnce,
                    retain: true,
                    payload: received_publication.payload,
                };
                publish_handle.publish(publication).await?;
            }
            TestScenario::Initiate => info!(
                "reporting result for received publication {:?}",
                received_publication.payload
            ),
        }
    }
}

async fn publish_forward_messages(mut publish_handle: PublishHandle) -> Result<()> {
    let mut seq_num = 0;
    loop {
        info!("publishing message {} to upstream broker", seq_num);
        let publication = Publication {
            topic_name: "forwards/1".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::from(seq_num.to_string()),
        };
        publish_handle.publish(publication).await?;

        info!("waiting for message send delay");
        time::delay_for(Duration::from_secs(1)).await;

        seq_num += 1;
    }
}

fn init_logging() {
    let subscriber = Subscriber::builder().with_max_level(Level::TRACE).finish();
    let _ = subscriber::set_global_default(subscriber);
}
