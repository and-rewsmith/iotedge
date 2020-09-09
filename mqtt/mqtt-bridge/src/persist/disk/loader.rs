use std::{pin::Pin, sync::Arc, task::Context, task::Poll, vec::IntoIter};

use futures_util::stream::Stream;
use mqtt3::proto::Publication;
use parking_lot::{Mutex, MutexGuard};

/// Message loader used to extract elements from bridge persistence
/// This component is responsible for message extraction from the persistence
/// It works by grabbing a snapshot of the most important messages from the persistence
/// Then, will return these elements in order
/// When the batch is exhausted it will grab a new batch
pub struct DiskMessageLoader {}

impl DiskMessageLoader {}

impl Stream for DiskMessageLoader {
    type Item = (Key, Publication);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {}
}

#[cfg(test)]
mod tests {}
