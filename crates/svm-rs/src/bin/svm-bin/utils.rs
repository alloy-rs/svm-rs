use std::future::Future;

/// Runs the `future` in a new [`tokio::runtime::Runtime`]
#[allow(unused)]
pub fn block_on<F: Future>(future: F) -> F::Output {
    let rt = tokio::runtime::Runtime::new().expect("could not start tokio rt");
    rt.block_on(future)
}
