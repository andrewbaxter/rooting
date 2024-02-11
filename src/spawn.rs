use futures::{
    channel::oneshot::channel,
    future::FusedFuture,
    select,
};
use wasm_bindgen_futures::spawn_local;
use crate::{
    own::{
        ScopeValue,
        defer,
    },
};

/// Spawn a background task that's canceled when the returned scope value is
/// dropped. You can use this to attach background tasks to elements that are
/// stopped when the element is removed.
pub fn spawn_rooted(mut f: impl Unpin + FusedFuture<Output = ()> + 'static) -> ScopeValue {
    let (cancel_tx, mut cancel_rx) = channel();
    spawn_local(async move {
        select!{
            _ = cancel_rx => {
            },
            _ = f => {
            }
        }
    });
    return defer(move || {
        _ = cancel_tx.send(());
    });
}
