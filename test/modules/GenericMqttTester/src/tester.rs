#![allow(unused_imports)]
use std::sync::Arc;

use bytes::Bytes;
use future::{join_all, Either};
use futures_util::{future, pin_mut, stream::StreamExt, stream::TryStreamExt};
use mpsc::UnboundedSender;
use time::Duration;
use tokio::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
    task::JoinHandle,
    time,
};
use tracing::info;

use mqtt3::{
    proto::{Publication, QoS},
    Client, Event, PublishHandle, ReceivedPublication, UpdateSubscriptionHandle,
};
use mqtt_broker_tests_util::client;
use mqtt_util::client_io::ClientIoSource;

use crate::{
    message_handler::{MessageHandler, RelayingMessageHandler, ReportResultMessageHandler},
    settings::{Settings, TestScenario},
    MessageTesterError, ShutdownHandle,
};

#[derive(Debug, Clone)]
pub struct MessageTesterShutdownHandle {
    poll_client_shutdown: Sender<()>,
    send_messages_shutdown: Sender<()>,
}

impl MessageTesterShutdownHandle {
    fn new(poll_client_shutdown: Sender<()>, send_messages_shutdown: Sender<()>) -> Self {
        Self {
            poll_client_shutdown,
            send_messages_shutdown,
        }
    }

    async fn shutdown(mut self) -> Result<(), MessageTesterError> {
        self.poll_client_shutdown
            .send(())
            .await
            .map_err(MessageTesterError::SendShutdownSignal)?;
        self.send_messages_shutdown
            .send(())
            .await
            .map_err(MessageTesterError::SendShutdownSignal)?;
        Ok(())
    }
}

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
    message_handler: Box<dyn MessageHandler>,
    shutdown_handle: MessageTesterShutdownHandle,
    poll_client_shutdown_recv: Receiver<()>,
    message_loop_shutdown_recv: Receiver<()>,
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

        let (poll_client_shutdown_send, poll_client_shutdown_recv) = mpsc::channel::<()>(1);
        let (message_loop_shutdown_send, message_loop_shutdown_recv) = mpsc::channel::<()>(1);
        let shutdown_handle =
            MessageTesterShutdownHandle::new(poll_client_shutdown_send, message_loop_shutdown_send);

        let client_sub_handle = client
            .update_subscription_handle()
            .map_err(MessageTesterError::UpdateSubscriptionHandle)?;
        Self::subscribe(client_sub_handle, settings.clone())?;

        Ok(Self {
            settings,
            client,
            publish_handle,
            message_handler,
            shutdown_handle,
            poll_client_shutdown_recv,
            message_loop_shutdown_recv,
        })
    }

    pub async fn run(self) -> Result<(), MessageTesterError> {
        let message_send_handle = self.message_handler.publication_sender_handle();
        let poll_client = tokio::spawn(poll_client(
            message_send_handle,
            self.client,
            self.poll_client_shutdown_recv,
        ));

        let message_handler = tokio::spawn(self.message_handler.run());

        let mut message_loop: Option<JoinHandle<Result<(), MessageTesterError>>> = None;
        if let TestScenario::Initiate = self.settings.test_scenario() {
            message_loop = Some(tokio::spawn(send_initial_messages(
                self.publish_handle.clone(),
                self.message_loop_shutdown_recv,
            )));
        }

        let mut tasks = vec![message_handler, poll_client];
        if let Some(message_loop) = message_loop {
            tasks.push(message_loop);
        }

        let task_statuses = join_all(tasks).await;
        for task in task_statuses {
            task.map_err(MessageTesterError::WaitForShutdown)??;
        }

        Ok(())
    }

    pub fn shutdown_handle(&self) -> MessageTesterShutdownHandle {
        self.shutdown_handle.clone()
    }

    fn subscribe(
        client_sub_handle: UpdateSubscriptionHandle,
        settings: Settings,
    ) -> Result<(), MessageTesterError> {
        todo!()
    }
}

async fn send_initial_messages(
    mut publish_handle: PublishHandle,
    mut shutdown_recv: Receiver<()>,
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

        let shutdown_recv_fut = shutdown_recv.next();
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
    mut client: Client<ClientIoSource>,
    mut shutdown_recv: Receiver<()>,
) -> Result<(), MessageTesterError> {
    loop {
        let message_send_handle = message_send_handle.clone();
        let event = client.try_next();
        let shutdown = shutdown_recv.next();
        match future::select(event, shutdown).await {
            Either::Left((event, _)) => {
                if let Ok(Some(event)) = event {
                    handle_event(event, message_send_handle)?;
                }
            }
            Either::Right((shutdown, _)) => {
                break;
            }
        }
    }

    Ok(())
}

fn handle_event(
    event: Event,
    message_send_handle: UnboundedSender<ReceivedPublication>,
) -> Result<(), MessageTesterError> {
    match event {
        Event::NewConnection { .. } => {
            info!("received new connection");
        }
        Event::Publication(publication) => {
            info!("received publication");
            message_send_handle
                .send(publication)
                .map_err(MessageTesterError::SendPublicationInChannel)?;
        }
        Event::SubscriptionUpdates(_) => {
            info!("received subscription update");
        }
        Event::Disconnected(_) => {
            info!("received disconnect");
        }
    };

    Ok(())
}
