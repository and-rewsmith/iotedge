use async_trait::async_trait;
use futures_util::{
    future::{self, Either},
    stream::StreamExt,
};
use mpsc::{Receiver, Sender, UnboundedReceiver, UnboundedSender};
use tokio::sync::mpsc;

use mqtt3::{
    proto::{Publication, QoS},
    PublishHandle, ReceivedPublication,
};

use crate::{MessageTesterError, BACKWARDS_TOPIC};

#[derive(Debug, Clone)]
pub struct MessageHandlerShutdownHandle(Sender<()>);

impl MessageHandlerShutdownHandle {
    pub fn new(sender: Sender<()>) -> Self {
        Self(sender)
    }

    pub async fn shutdown(mut self) -> Result<(), MessageTesterError> {
        self.0
            .send(())
            .await
            .map_err(MessageTesterError::SendShutdownSignal)
    }
}

/// Responsible for receiving publications and taking some action.
#[async_trait]
pub trait MessageHandler {
    /// Starts handling messages sent to the handler
    async fn run(mut self: Box<Self>) -> Result<(), MessageTesterError>;

    /// Sends a publication to be handled by the message handler
    fn publication_sender_handle(&self) -> UnboundedSender<ReceivedPublication>;

    // Get the shutdown handle to stop the run() method
    fn shutdown_handle(&self) -> MessageHandlerShutdownHandle;
}

/// Responsible for receiving publications and reporting result to the TRC.
pub struct ReportResultMessageHandler {}

impl ReportResultMessageHandler {
    pub fn new() -> Self {
        todo!()
    }
}

#[async_trait]
impl MessageHandler for ReportResultMessageHandler {
    async fn run(mut self: Box<Self>) -> Result<(), MessageTesterError> {
        todo!()
    }

    fn publication_sender_handle(&self) -> UnboundedSender<ReceivedPublication> {
        todo!()
    }

    fn shutdown_handle(&self) -> MessageHandlerShutdownHandle {
        todo!()
    }
}

/// Responsible for receiving publications and sending them back to the downstream edge.
pub struct RelayingMessageHandler {
    publication_sender: UnboundedSender<ReceivedPublication>,
    publication_receiver: UnboundedReceiver<ReceivedPublication>,
    shutdown_handle: MessageHandlerShutdownHandle,
    shutdown_recv: Receiver<()>,
    publish_handle: PublishHandle,
}

impl RelayingMessageHandler {
    pub fn new(publish_handle: PublishHandle) -> Self {
        let (publication_sender, publication_receiver) =
            mpsc::unbounded_channel::<ReceivedPublication>();
        let (shutdown_send, shutdown_recv) = mpsc::channel::<()>(1);
        let shutdown_handle = MessageHandlerShutdownHandle::new(shutdown_send);

        Self {
            publication_sender,
            publication_receiver,
            shutdown_handle,
            shutdown_recv,
            publish_handle,
        }
    }
}

#[async_trait]
impl MessageHandler for RelayingMessageHandler {
    async fn run(mut self: Box<Self>) -> Result<(), MessageTesterError> {
        loop {
            let received_pub = self.publication_receiver.next();
            let shutdown_signal = self.shutdown_recv.next();

            match future::select(received_pub, shutdown_signal).await {
                Either::Left((received_pub, _)) => {
                    if let Some(received_pub) = received_pub {
                        let new_publication = Publication {
                            topic_name: BACKWARDS_TOPIC.to_string(),
                            qos: QoS::ExactlyOnce,
                            retain: true,
                            payload: received_pub.payload,
                        };
                        self.publish_handle
                            .publish(new_publication)
                            .await
                            .map_err(MessageTesterError::Publish)?;
                    } else {
                        return Err(MessageTesterError::ListenForIncomingPublications);
                    }
                }
                Either::Right((shutdown_signal, _)) => {
                    if let Some(shutdown_signal) = shutdown_signal {
                        return Ok(());
                    } else {
                        return Err(MessageTesterError::ListenForShutdown);
                    }
                }
            };
        }
    }

    fn publication_sender_handle(&self) -> UnboundedSender<ReceivedPublication> {
        self.publication_sender.clone()
    }

    fn shutdown_handle(&self) -> MessageHandlerShutdownHandle {
        self.shutdown_handle.clone()
    }
}
