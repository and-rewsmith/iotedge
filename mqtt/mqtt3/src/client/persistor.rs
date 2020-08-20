use async_trait;
pub trait Persistor {
    async fn persist(publication: crate::proto::Publication);
}
