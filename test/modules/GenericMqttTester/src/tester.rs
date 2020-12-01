use thiserror;
use tokio::task::JoinHandle;

pub struct MessageTesterShutdownHandle {}

impl MessageTesterShutdownHandle {
    pub fn shutdown() -> Result<(), MessageTesterError> {
        Ok(())
    }
}

pub struct MessageTester {}

impl MessageTester {
    pub fn new() -> Self {
        Self {}
    }

    pub fn run() -> (
        JoinHandle<Result<(), MessageTesterError>>,
        MessageTesterShutdownHandle,
    ) {
        todo!()
    }
}

trait MessageHandler {
    fn handle_publication() -> Result<(), MessageTesterError>;
}

struct ReportResultMessageHandler {}

impl MessageHandler for ReportResultMessageHandler {
    fn handle_publication() -> Result<(), MessageTesterError> {
        Ok(())
    }
}

struct SendBackMessageHandler {}

impl MessageHandler for SendBackMessageHandler {
    fn handle_publication() -> Result<(), MessageTesterError> {
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MessageTesterError {
    // #[error("could not parse URL: {0}. {1}")]
// Placeholder(String, #[source] ParseError),
}
