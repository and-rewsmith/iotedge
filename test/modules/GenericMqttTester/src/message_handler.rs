use futures_util::{
    future::{select, Either},
    stream::StreamExt,
};
use mpsc::{Receiver, UnboundedReceiver, UnboundedSender};
use tokio::{sync::mpsc, task::JoinHandle};

use mqtt3::{
    proto::{Publication, QoS},
    PublishHandle, ReceivedPublication,
};

use crate::{MessageTesterError, ShutdownHandle};

const RELAY_TOPIC: &str = "backwards/1";

/// Responsible for receiving publications and taking some action.
pub trait MessageHandler {
    /// Starts handling messages sent to the handler
    fn run(self) -> (JoinHandle<Result<(), MessageTesterError>>, ShutdownHandle);

    /// Sends a publication to be handled by the message handler
    fn publication_sender_handle(&self) -> UnboundedSender<ReceivedPublication>;
}

/// Responsible for receiving publications and reporting result to the TRC.
pub struct ReportResultMessageHandler {}

impl ReportResultMessageHandler {
    pub fn new() -> Self {
        todo!()
    }
}

impl MessageHandler for ReportResultMessageHandler {
    fn run(self) -> (JoinHandle<Result<(), MessageTesterError>>, ShutdownHandle) {
        todo!()
    }

    fn publication_sender_handle(&self) -> UnboundedSender<ReceivedPublication> {
        todo!()
    }
}

/// Responsible for receiving publications and sending them back to the downstream edge.
pub struct RelayingMessageHandler {
    publication_sender: UnboundedSender<ReceivedPublication>,
    publication_receiver: UnboundedReceiver<ReceivedPublication>,
    shutdown_handle: ShutdownHandle,
    shutdown_recv: Receiver<()>,
    publish_handle: PublishHandle,
}

impl RelayingMessageHandler {
    pub fn new(publish_handle: PublishHandle) -> Self {
        let (publication_sender, publication_receiver) =
            mpsc::unbounded_channel::<ReceivedPublication>();
        let (shutdown_send, shutdown_recv) = mpsc::channel::<()>(1);
        let shutdown_handle = ShutdownHandle::new(shutdown_send);

        Self {
            publication_sender,
            publication_receiver,
            shutdown_handle,
            shutdown_recv,
            publish_handle,
        }
    }

    async fn relay_message(mut self) -> Result<(), MessageTesterError> {
        loop {
            let received_pub = self.publication_receiver.next();
            let shutdown_signal = self.shutdown_recv.next();

            match select(received_pub, shutdown_signal).await {
                Either::Left((received_pub, _)) => {
                    if let Some(received_pub) = received_pub {
                        let new_publication = Publication {
                            topic_name: RELAY_TOPIC.to_string(),
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
}

impl MessageHandler for RelayingMessageHandler {
    fn run(self) -> (JoinHandle<Result<(), MessageTesterError>>, ShutdownHandle) {
        let shutdown_handle = self.shutdown_handle.clone();
        let relay_message_join_handle = tokio::spawn(self.relay_message());

        (relay_message_join_handle, shutdown_handle)
    }

    fn publication_sender_handle(&self) -> UnboundedSender<ReceivedPublication> {
        self.publication_sender.clone()
    }
}
