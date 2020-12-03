use mpsc::{Receiver, UnboundedSender};
use tokio::{sync::mpsc, task::JoinHandle};

use mqtt3::{Client, PublishHandle, ReceivedPublication};
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
    shutdown_recv: Receiver<()>,
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

    pub fn run(self) -> Result<(), MessageTesterError> {
        let mut message_loop: Option<JoinHandle<Result<(), MessageTesterError>>> = None;
        if let TestScenario::Initiate = self.settings.test_scenario() {
            message_loop = Some(tokio::spawn(initiate_message_sending(
                self.publish_handle.clone(),
            )));
        }

        let message_send_handle = self.message_handler.publication_sender_handle();
        let poll_client = tokio::spawn(poll_client(message_send_handle, self.client));

        // shutdown

        Ok(())
    }

    pub fn shutdown_handle(&self) -> ShutdownHandle {
        self.shutdown_handle.clone()
    }
}

async fn initiate_message_sending(publish_handle: PublishHandle) -> Result<(), MessageTesterError> {
    todo!()
}

async fn poll_client(
    message_send_handle: UnboundedSender<ReceivedPublication>,
    client: Client<ClientIoSource>,
) -> Result<(), MessageTesterError> {
    todo!()
}
