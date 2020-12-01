use mpsc::Receiver;
use tokio::{sync::mpsc, task::JoinHandle};

use mqtt3::{Client, PublishHandle};
use mqtt_broker_tests_util::client;
use mqtt_util::client_io::ClientIoSource;

use crate::settings::Settings;
use crate::{
    message_handler::{MessageHandler, ReportResultMessageHandler, SendBackMessageHandler},
    settings::TestScenario,
    MessageTesterError, ShutdownHandle,
};

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
