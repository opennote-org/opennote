/// Run async codes in sync functions
pub fn run_async_code<F, R>(closure: F) -> R
where
    F: Future<Output = R>,
{
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(closure))
}
