use async_trait::async_trait;

#[async_trait]
pub trait VectorDatabase<T> {
    fn get_client(&self) -> &T;
}
