use mqtt3::{Client, PublishError, PublishHandle, ReceivedPublication};
use mqtt_broker_tests_util::client;
use mqtt_util::client_io::ClientIoSource;
use thiserror;
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

use crate::settings::Settings;
use crate::settings::TestScenario;
use mpsc::{error::SendError, Receiver, Sender, UnboundedReceiver, UnboundedSender};

#[derive(Debug, Clone)]
pub struct ShutdownHandle(Sender<()>);

impl ShutdownHandle {
    pub fn new(sender: Sender<()>) -> Self {
        Self(sender)
    }

    pub async fn shutdown(mut self) -> Result<(), MessageTesterError> {
        self.0
            .send(())
            .await
            .map_err(MessageTesterError::ShutdownMessageHandler)
    }
}

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
        let client = client::create_client_from_module_env();
        let publish_handle = client
            .publish_handle()
            .map_err(MessageTesterError::PublishHandle)?;

        // need to handle conditional new thread for sending messages. need to integrate into shutdown
        let message_handler: Box<dyn MessageHandler> = match settings.test_scenario() {
            TestScenario::Send => Box::new(SendBackMessageHandler::new()),
            TestScenario::Receive => Box::new(ReportResultMessageHandler::new()),
        };

        let (shutdown_send, shutdown_recv) = mpsc::channel::<()>(1);
        let shutdown_handle = ShutdownHandle::new(shutdown_send);

        Ok(Self {
            settings,
            client,
            publish_handle,
            shutdown_handle,
            message_handler,
            shutdown_recv,
        })
    }

    pub fn run() -> (JoinHandle<Result<(), MessageTesterError>>, ShutdownHandle) {
        todo!()
    }
}

pub trait MessageHandler {
    fn handle_publication(&self) -> Result<(), MessageTesterError>;

    fn shutdown_handle(&self) -> ShutdownHandle;
}

pub struct ReportResultMessageHandler {}

impl ReportResultMessageHandler {
    pub fn new() -> Self {
        Self {}
    }
}

impl MessageHandler for ReportResultMessageHandler {
    fn handle_publication(&self) -> Result<(), MessageTesterError> {
        Ok(())
    }

    fn shutdown_handle(&self) -> ShutdownHandle {
        todo!()
    }
}

pub struct SendBackMessageHandler {
    publication_sender: UnboundedSender<ReceivedPublication>,
    shutdown_handle: ShutdownHandle,
    message_send_join_handle: JoinHandle<Result<(), MessageTesterError>>,
}

impl SendBackMessageHandler {
    pub fn new() -> Self {
        // start thread that will listen
        let (publication_sender, publication_receiver) =
            mpsc::unbounded_channel::<ReceivedPublication>();
        let (shutdown_send, shutdown_recv) = mpsc::channel::<()>(1);
        let shutdown_handle = ShutdownHandle::new(shutdown_send);

        let message_send_join_handle = tokio::spawn(Self::send_messages_back(
            publication_receiver,
            shutdown_recv,
        ));

        Self {
            publication_sender,
            shutdown_handle,
            message_send_join_handle,
        }
    }

    async fn send_messages_back(
        publication_receiver: UnboundedReceiver<ReceivedPublication>,
        shutdown_recv: Receiver<()>,
    ) -> Result<(), MessageTesterError> {
        // select on pub receiver and shutdown receive to send messages until shutdown
        todo!()
    }
}

impl MessageHandler for SendBackMessageHandler {
    fn handle_publication(&self) -> Result<(), MessageTesterError> {
        Ok(())
    }

    fn shutdown_handle(&self) -> ShutdownHandle {
        self.shutdown_handle.clone()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MessageTesterError {
    #[error("could not get client publish handle")]
    PublishHandle(#[source] PublishError),

    #[error("could not get client publish handle")]
    ShutdownMessageHandler(#[source] SendError<()>),
}
