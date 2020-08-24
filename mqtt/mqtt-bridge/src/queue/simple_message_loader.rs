use std::cmp::min;

use anyhow::Result;
use core::slice::Iter;
use mqtt3::proto::Publication;

use crate::queue::{MessageLoader, QueueError};

pub struct SimpleMessageLoader {
    messages: Vec<(String, Publication)>,
}

impl SimpleMessageLoader {
    pub fn new(messages: Vec<(String, Publication)>) -> Self {
        SimpleMessageLoader { messages }
    }
}

impl<'a> MessageLoader<'a> for SimpleMessageLoader {
    type Iter = Iter<'a, (String, Publication)>;

    fn range(&'a self, count: usize) -> Result<Iter<'a, (String, Publication)>> {
        let output_cardinality = min(self.messages.len(), count);

        Ok(self
            .messages
            .get(0..output_cardinality)
            .ok_or(QueueError::LoadMessage())?
            .into_iter())
    }
}
