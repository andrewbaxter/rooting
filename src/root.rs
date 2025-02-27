//! These methods root scoped values. Each one uses a different reference point as
//! the root, so mixing them may leave html elements with references to freed
//! values.  Generally try to stick with `set_root` or `set_root_non_dom`, and
//! don't mix them.
use {
    crate::{
        scope_any,
        El,
        ScopeValue,
    },
    gloo_utils::document,
    std::cell::Cell,
    wasm_bindgen::UnwrapThrowExt,
};

thread_local!{
    static ROOT: Cell<ScopeValue> = Cell::new(scope_any(()));
}

/// Replaces the existing element with id `id`, taking ownership and extending the
/// new element's lifetime.
pub fn set_root_replace(id: &str, el: El) {
    document().get_element_by_id(id).unwrap().replace_with_with_node_1(&el.0.borrow().el).unwrap_throw();
    ROOT.with(|r| r.set(scope_any(el)));
}

/// Sets the elements as the children of the body, taking ownership and their
/// lifetimes.
pub fn set_root(elements: Vec<El>) {
    document()
        .body()
        .unwrap()
        .replace_children_with_node(&elements.iter().map(|e| e.0.borrow().el.clone()).collect());
    ROOT.with(|r| r.set(scope_any(elements)));
}

/// Roots the lifetime of some values with no relation to the DOM, for (ex:)
/// scripts injecting values into other documents.
pub fn set_root_non_dom(value: ScopeValue) {
    ROOT.with(|r| r.set(value));
}
