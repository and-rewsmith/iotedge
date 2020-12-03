#![allow(unused_imports)]
use std::sync::Arc;

use bytes::Bytes;
use future::Either;
use futures_util::{future, pin_mut, stream::StreamExt};
use mpsc::{Receiver, UnboundedSender};
use time::Duration;
use tokio::{
    sync::{mpsc, Mutex},
    task::JoinHandle,
    time,
};
use tracing::info;

use mqtt3::{
    proto::{Publication, QoS},
    Client, PublishHandle, ReceivedPublication,
};
use mqtt_broker_tests_util::client;
use mqtt_util::client_io::ClientIoSource;

use crate::{
    message_handler::{MessageHandler, RelayingMessageHandler, ReportResultMessageHandler},
    settings::{Settings, TestScenario},
    MessageTesterError, ShutdownHandle,
};

/// Abstracts the test logic for this generic mqtt telemetry test module.
/// It will run in one of two modes. The behavior of this struct depends on this mode.
///
/// 1: Test module runs on the lowest node in the topology.
///     - Spawn a thread that publishes messages continuously to upstream edge.
///     - Receives same message routed back from upstream edge and reports the result to the TRC.
///
/// 2: Test module runs on middle node in the topology.
///     - Receives a message from downstream edge and relays it back to downstream edge.
pub struct MessageTester {
    settings: Settings,
    client: Client<ClientIoSource>,
    publish_handle: PublishHandle,
    shutdown_handle: ShutdownHandle,
    shutdown_recv: Arc<Mutex<Receiver<()>>>,
    message_handler: Box<dyn MessageHandler>,
}

impl MessageTester {
    pub fn new(settings: Settings) -> Result<Self, MessageTesterError> {
        let client = client::create_client_from_module_env()
            .map_err(MessageTesterError::ParseEnvironment)?;
        let publish_handle = client
            .publish_handle()
            .map_err(MessageTesterError::PublishHandle)?;

        let message_handler: Box<dyn MessageHandler> = match settings.test_scenario() {
            TestScenario::Initiate => Box::new(RelayingMessageHandler::new(publish_handle.clone())),
            TestScenario::Relay => Box::new(ReportResultMessageHandler::new()),
        };

        let (shutdown_send, shutdown_recv) = mpsc::channel::<()>(1);
        let shutdown_recv = Arc::new(Mutex::new(shutdown_recv));
        let shutdown_handle = ShutdownHandle::new(shutdown_send);

        // wait for subscriptions

        Ok(Self {
            settings,
            client,
            publish_handle,
            shutdown_handle,
            message_handler,
            shutdown_recv,
        })
    }

    pub async fn run(self) -> Result<(), MessageTesterError> {
        let mut message_loop: Option<JoinHandle<Result<(), MessageTesterError>>> = None;
        if let TestScenario::Initiate = self.settings.test_scenario() {
            message_loop = Some(tokio::spawn(send_initial_messages(
                self.publish_handle.clone(),
                self.shutdown_recv.clone(),
            )));
        }

        let message_send_handle = self.message_handler.publication_sender_handle();
        let poll_client = tokio::spawn(poll_client(
            message_send_handle,
            self.client,
            self.shutdown_recv.clone(),
        ));

        match message_loop {
            None => poll_client
                .await
                .map_err(MessageTesterError::PollClientThreadPanic)?,
            Some(message_loop) => match future::select(message_loop, poll_client).await {
                Either::Left((message_loop, poll_client)) => {
                    poll_client
                        .await
                        .map_err(MessageTesterError::PollClientThreadPanic)??;
                    message_loop.map_err(MessageTesterError::SendMessageLoopThreadPanic)?
                }
                Either::Right((poll_client, message_loop)) => {
                    message_loop
                        .await
                        .map_err(MessageTesterError::SendMessageLoopThreadPanic)??;
                    poll_client.map_err(MessageTesterError::PollClientThreadPanic)?
                }
            },
        }
    }

    pub fn shutdown_handle(&self) -> ShutdownHandle {
        self.shutdown_handle.clone()
    }
}

async fn send_initial_messages(
    mut publish_handle: PublishHandle,
    shutdown_recv: Arc<Mutex<Receiver<()>>>,
) -> Result<(), MessageTesterError> {
    let mut seq_num: u32 = 0;
    loop {
        info!("publishing message {} to upstream broker", seq_num);
        let publication = Publication {
            topic_name: "forwards/1".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::from(seq_num.to_string()),
        };

        let mut shutdown_recv_lock = shutdown_recv.lock().await;
        let shutdown_recv_fut = shutdown_recv_lock.next();
        let publish_fut = publish_handle.publish(publication);
        pin_mut!(publish_fut);

        match future::select(shutdown_recv_fut, publish_fut).await {
            Either::Left((shutdown, publish)) => {
                publish.await.map_err(MessageTesterError::Publish)?;
                shutdown.ok_or(MessageTesterError::ListenForShutdown)?;
                break;
            }
            Either::Right((publish, _)) => {
                publish.map_err(MessageTesterError::Publish)?;
            }
        };

        info!("waiting for message send delay");
        time::delay_for(Duration::from_secs(1)).await;

        seq_num += 1;
    }

    Ok(())
}

async fn poll_client(
    message_send_handle: UnboundedSender<ReceivedPublication>,
    client: Client<ClientIoSource>,
    shutdown_recv: Arc<Mutex<Receiver<()>>>,
) -> Result<(), MessageTesterError> {
    todo!()
}
