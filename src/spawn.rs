use {
    futures::{
        Future,
        FutureExt,
        channel::oneshot::{
            Receiver,
            channel,
        },
        select,
    },
    wasm_bindgen_futures::spawn_local,
};

/// Spawn a background task that's canceled when the returned scope value is
/// dropped. You can use this to attach background tasks to elements that are
/// stopped when the element is removed.
pub fn spawn_rooted<T: 'static>(f: impl Future<Output = T> + 'static) -> Receiver<T> {
    let (mut complete_tx, complete_rx) = channel();
    let f = Box::pin(f);
    spawn_local(async move {
        let cancel_rx = complete_tx.cancellation();
        select!{
            _ = cancel_rx.fuse() => {
            },
            r = f.fuse() => {
                _ = complete_tx.send(r);
            }
        }
    });
    return complete_rx;
}
