use std::{collections::HashMap, convert::TryFrom, convert::TryInto, sync::Arc, time::Duration};

use async_trait::async_trait;
use mqtt3::{proto::Publication, Event, ReceivedPublication};
use mqtt_broker::TopicFilter;
use parking_lot::Mutex;
use parking_lot::MutexGuard;
use tracing::{debug, info, warn};

use crate::{
    client::{ClientConnectError, EventHandler, MqttClient},
    persist::{
        MessageLoader, PersistError, PublicationStore, StreamWakeableState, WakingMemoryStore,
    },
    settings::{ConnectionSettings, Credentials, TopicRule},
};

const BATCH_SIZE: usize = 10;

/// Bridge implementation that connects to local broker and remote broker and handles messages flow
pub struct Bridge {
    local_client: MqttClient<MessageHandler<WakingMemoryStore>>,
    remote_client: MqttClient<MessageHandler<WakingMemoryStore>>,
    local_subs: Vec<String>,
    remote_subs: Vec<String>,
    local_client_id: String,
    connection_settings: ConnectionSettings,
    system_address: String,
}

impl Bridge {
    // TODO PRE: create the abstraction that will handle the upstream pump and pass into the client
    pub fn new(
        system_address: String,
        device_id: String,
        connection_settings: ConnectionSettings,
    ) -> Result<Self, BridgeError> {
        let forwards: HashMap<String, TopicRule> = connection_settings
            .forwards()
            .iter()
            .map(|sub| Self::format_key_value(sub))
            .collect();

        let subscriptions: HashMap<String, TopicRule> = connection_settings
            .subscriptions()
            .iter()
            .map(|sub| Self::format_key_value(sub))
            .collect();

        let outgoing_persist = PublicationStore::new_memory(BATCH_SIZE);
        let outgoing_loader = outgoing_persist.loader();
        let incoming_persist = PublicationStore::new_memory(BATCH_SIZE);
        let incoming_loader = incoming_persist.loader();

        let (remote_subs, topic_rules): (Vec<_>, Vec<_>) = subscriptions.drain().unzip();
        let topic_filters = topic_rules
            .into_iter()
            .map(|topic| topic.try_into())
            .collect::<Result<Vec<_>, _>>()?;
        let remote_client = MqttClient::new(
            connection_settings.address(),
            Some(connection_settings.port().to_owned()),
            connection_settings.keep_alive(),
            connection_settings.clean_session(),
            MessageHandler::new(incoming_persist, topic_filters),
            connection_settings.credentials(),
            true,
        );

        let (local_subs, topic_rules): (Vec<_>, Vec<_>) = forwards.drain().unzip();
        let topic_filters = topic_rules
            .into_iter()
            .map(|topic| topic.try_into())
            .collect::<Result<Vec<_>, _>>()?;
        let local_client_id = format!(
            "{}/$edgeHub/$bridge/{}",
            device_id,
            connection_settings.name()
        );
        let local_client = MqttClient::new(
            system_address.as_str(),
            Some(connection_settings.port().to_owned()),
            connection_settings.keep_alive(),
            connection_settings.clean_session(),
            MessageHandler::new(outgoing_persist, topic_filters),
            &Credentials::Anonymous(local_client_id),
            true,
        );

        Ok(Self {
            local_client,
            remote_client,
            local_subs,
            remote_subs,
            local_client_id,
            connection_settings,
            system_address,
        })
    }

    fn format_key_value(topic: &TopicRule) -> (String, TopicRule) {
        let key = if let Some(local) = topic.local() {
            format!("{}/{}", local, topic.pattern().to_string())
        } else {
            topic.pattern().into()
        };
        (key, topic.clone())
    }

    pub async fn start(&mut self) -> Result<(), BridgeError> {
        info!("Starting bridge...{}", self.connection_settings.name());

        self.connect_to_local().await?;
        self.connect_to_remote().await?;

        Ok(())
    }

    async fn connect_to_remote(&mut self) -> Result<(), BridgeError> {
        info!(
            "connecting to remote broker {}",
            self.connection_settings.address()
        );

        debug!("subscribing to remote {:?}", self.remote_subs);
        self.local_client
            .subscribe(&self.remote_subs)
            .await
            .map_err(BridgeError::Subscribe)?;

        //TODO: handle this with shutdown
        let _events_task = tokio::spawn(self.remote_client.handle_events());

        // TODO REMVE
        Ok(())
    }

    async fn connect_to_local(&mut self) -> Result<(), BridgeError> {
        info!(
            "connecting to local broker {}, clientid {}",
            self.system_address, self.local_client_id
        );

        debug!("subscribing to local {:?}", self.local_subs);
        self.local_client
            .subscribe(&self.local_subs)
            .await
            .map_err(BridgeError::Subscribe)?;

        //TODO: handle this with shutdown
        let _events_task = tokio::spawn(self.local_client.handle_events());

        Ok(())
    }
}

#[derive(Clone)]
struct TopicMapper {
    topic_settings: TopicRule,
    topic_filter: TopicFilter,
}

impl TryFrom<TopicRule> for TopicMapper {
    type Error = BridgeError;

    fn try_from(topic: TopicRule) -> Result<Self, BridgeError> {
        let topic_filter = topic
            .pattern()
            .parse()
            .map_err(BridgeError::TopicFilterParse)?;

        Ok(Self {
            topic_settings: topic,
            topic_filter,
        })
    }
}

/// Handle events from client and saves them with the forward topic
// TODO PRE: make inner a locked version of the publication store
struct MessageHandler<S>
where
    S: StreamWakeableState,
{
    topic_mappers: Vec<TopicMapper>,
    inner: PublicationStore<S>,
}

impl<S> MessageHandler<S>
where
    S: StreamWakeableState,
{
    pub fn new(persistor: PublicationStore<S>, topic_mappers: Vec<TopicMapper>) -> Self {
        Self {
            topic_mappers,
            inner: persistor,
        }
    }

    fn transform(&self, topic_name: &str) -> Option<String> {
        self.topic_mappers.iter().find_map(|mapper| {
            mapper
                .topic_settings
                .local()
                // maps if local does not have a value it uses the topic that was received,
                // else it checks that the received topic starts with local prefix and removes the local prefix
                .map_or(Some(topic_name), |local_prefix| {
                    let prefix = format!("{}/", local_prefix);
                    topic_name.strip_prefix(&prefix)
                })
                // match topic without local prefix with the topic filter pattern
                .filter(|stripped_topic| mapper.topic_filter.matches(stripped_topic))
                .map(|stripped_topic| {
                    if let Some(remote_prefix) = mapper.topic_settings.remote() {
                        format!("{}/{}", remote_prefix, stripped_topic)
                    } else {
                        stripped_topic.to_string()
                    }
                })
        })
    }
}

// TODO: implement for generic
#[async_trait]
impl EventHandler for MessageHandler<WakingMemoryStore> {
    type Error = BridgeError;

    async fn handle_event(&mut self, event: Event) -> Result<(), Self::Error> {
        if let Event::Publication(publication) = event {
            let ReceivedPublication {
                topic_name,
                qos,
                retain,
                payload,
                dup: _,
            } = publication;
            let forward_publication = self.transform(topic_name.as_ref()).map(|f| Publication {
                topic_name: f,
                qos,
                retain,
                payload,
            });

            if let Some(f) = forward_publication {
                debug!("Save message to store");
                self.inner.push(f).map_err(BridgeError::Store)?;
            } else {
                warn!("No topic matched");
            }
        }

        Ok(())
    }
}

/// Authentication error.
#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("failed to save to store.")]
    Store(#[from] PersistError),

    #[error("failed to subscribe to topic.")]
    Subscribe(#[from] ClientConnectError),

    #[error("failed to parse topic pattern.")]
    TopicFilterParse(#[from] mqtt_broker::Error),

    #[error("failed to load settings.")]
    LoadingSettings(#[from] config::ConfigError),
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use futures_util::stream::StreamExt;
    use std::str::FromStr;

    use mqtt3::{
        proto::{Publication, QoS},
        Event, ReceivedPublication,
    };
    use mqtt_broker::TopicFilter;

    use crate::bridge::{Bridge, MessageHandler, TopicMapper};
    use crate::client::EventHandler;
    use crate::persist::PublicationStore;
    use crate::settings::Settings;

    #[tokio::test]
    async fn bridge_new() {
        let settings = Settings::from_file("tests/config.json").unwrap();
        let connection_settings = settings.upstream().unwrap();

        let bridge = Bridge::new(
            "localhost:5555".into(),
            "d1".into(),
            connection_settings.clone(),
        );

        let (key, value) = bridge.forwards.get_key_value("temp/#").unwrap();
        assert_eq!(key, "temp/#");
        assert_eq!(value.remote().unwrap(), "floor/kitchen");
        assert_eq!(value.local(), None);

        let (key, value) = bridge.forwards.get_key_value("pattern/#").unwrap();
        assert_eq!(key, "pattern/#");
        assert_eq!(value.remote(), None);

        let (key, value) = bridge.forwards.get_key_value("local/floor/#").unwrap();
        assert_eq!(key, "local/floor/#");
        assert_eq!(value.local().unwrap(), "local");
        assert_eq!(value.remote().unwrap(), "remote");

        let (key, value) = bridge.subscriptions.get_key_value("temp/#").unwrap();
        assert_eq!(key, "temp/#");
        assert_eq!(value.remote().unwrap(), "floor/kitchen");
    }

    #[tokio::test]
    async fn message_handler_saves_message_with_local_and_forward_topic() {
        let batch_size: usize = 5;
        let settings = Settings::from_file("tests/config.json").unwrap();
        let connection_settings = settings.upstream().unwrap();

        let topics: Vec<TopicMapper> = connection_settings
            .forwards()
            .iter()
            .map(move |sub| TopicMapper {
                topic_settings: sub.clone(),
                topic_filter: TopicFilter::from_str(sub.pattern()).unwrap(),
            })
            .collect();

        let persistor = PublicationStore::new_memory(batch_size);
        let mut handler = MessageHandler::new(persistor, topics);

        let pub1 = ReceivedPublication {
            topic_name: "local/floor/1".to_string(),
            qos: QoS::AtLeastOnce,
            retain: true,
            payload: Bytes::new(),
            dup: false,
        };

        let expected = Publication {
            topic_name: "remote/floor/1".to_string(),
            qos: QoS::AtLeastOnce,
            retain: true,
            payload: Bytes::new(),
        };

        handler
            .handle_event(Event::Publication(pub1))
            .await
            .unwrap();

        let loader = handler.inner.loader();

        let extracted1 = loader.lock().next().await.unwrap();
        assert_eq!(extracted1.1, expected);
    }

    #[tokio::test]
    async fn message_handler_saves_message_with_forward_topic() {
        let batch_size: usize = 5;
        let settings = Settings::from_file("tests/config.json").unwrap();
        let connection_settings = settings.upstream().unwrap();

        let topics: Vec<TopicMapper> = connection_settings
            .forwards()
            .iter()
            .map(move |sub| TopicMapper {
                topic_settings: sub.clone(),
                topic_filter: TopicFilter::from_str(sub.pattern()).unwrap(),
            })
            .collect();

        let persistor = PublicationStore::new_memory(batch_size);
        let mut handler = MessageHandler::new(persistor, topics);

        let pub1 = ReceivedPublication {
            topic_name: "temp/1".to_string(),
            qos: QoS::AtLeastOnce,
            retain: true,
            payload: Bytes::new(),
            dup: false,
        };

        let expected = Publication {
            topic_name: "floor/kitchen/temp/1".to_string(),
            qos: QoS::AtLeastOnce,
            retain: true,
            payload: Bytes::new(),
        };

        handler
            .handle_event(Event::Publication(pub1))
            .await
            .unwrap();

        let loader = handler.inner.loader();

        let extracted1 = loader.lock().next().await.unwrap();
        assert_eq!(extracted1.1, expected);
    }

    #[tokio::test]
    async fn message_handler_saves_message_with_no_forward_mapping() {
        let batch_size: usize = 5;
        let settings = Settings::from_file("tests/config.json").unwrap();
        let connection_settings = settings.upstream().unwrap();

        let topics: Vec<TopicMapper> = connection_settings
            .forwards()
            .iter()
            .map(move |sub| TopicMapper {
                topic_settings: sub.clone(),
                topic_filter: TopicFilter::from_str(sub.pattern()).unwrap(),
            })
            .collect();

        let persistor = PublicationStore::new_memory(batch_size);
        let mut handler = MessageHandler::new(persistor, topics);

        let pub1 = ReceivedPublication {
            topic_name: "pattern/p1".to_string(),
            qos: QoS::AtLeastOnce,
            retain: true,
            payload: Bytes::new(),
            dup: false,
        };

        let expected = Publication {
            topic_name: "pattern/p1".to_string(),
            qos: QoS::AtLeastOnce,
            retain: true,
            payload: Bytes::new(),
        };

        handler
            .handle_event(Event::Publication(pub1))
            .await
            .unwrap();

        let loader = handler.inner.loader();

        let extracted1 = loader.lock().next().await.unwrap();
        assert_eq!(extracted1.1, expected);
    }

    #[tokio::test]
    async fn message_handler_no_topic_match() {
        let batch_size: usize = 5;
        let settings = Settings::from_file("tests/config.json").unwrap();
        let connection_settings = settings.upstream().unwrap();

        let topics: Vec<TopicMapper> = connection_settings
            .forwards()
            .iter()
            .map(move |sub| TopicMapper {
                topic_settings: sub.clone(),
                topic_filter: TopicFilter::from_str(sub.pattern()).unwrap(),
            })
            .collect();

        let persistor = PublicationStore::new_memory(batch_size);
        let mut handler = MessageHandler::new(persistor, topics);

        let pub1 = ReceivedPublication {
            topic_name: "local/temp/1".to_string(),
            qos: QoS::AtLeastOnce,
            retain: true,
            payload: Bytes::new(),
            dup: false,
        };

        handler
            .handle_event(Event::Publication(pub1))
            .await
            .unwrap();

        let loader = handler.inner.loader();

        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        futures_util::future::select(interval.next(), loader.lock().next()).await;
    }
}
