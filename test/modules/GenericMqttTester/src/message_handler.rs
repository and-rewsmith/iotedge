use mpsc::{Receiver, UnboundedReceiver, UnboundedSender};
use tokio::{sync::mpsc, task::JoinHandle};

use mqtt3::ReceivedPublication;

use crate::{MessageTesterError, ShutdownHandle};

pub trait MessageHandler {
    fn handle_publication(
        &self,
        publication: ReceivedPublication,
    ) -> Result<(), MessageTesterError>;

    fn shutdown_handle(&self) -> ShutdownHandle;
}

pub struct ReportResultMessageHandler {}

impl ReportResultMessageHandler {
    pub fn new() -> Self {
        todo!()
    }
}

impl MessageHandler for ReportResultMessageHandler {
    fn handle_publication(
        &self,
        publication: ReceivedPublication,
    ) -> Result<(), MessageTesterError> {
        todo!()
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
        todo!()
    }
}

impl MessageHandler for SendBackMessageHandler {
    fn handle_publication(
        &self,
        publication: ReceivedPublication,
    ) -> Result<(), MessageTesterError> {
        todo!()
    }

    fn shutdown_handle(&self) -> ShutdownHandle {
        self.shutdown_handle.clone()
    }
}
