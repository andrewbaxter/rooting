use std::cell::Cell;
use gloo_utils::document;
use wasm_bindgen::UnwrapThrowExt;
use crate::El;

thread_local!{
    static ROOT: Cell<Vec<El>> = Cell::new(vec![]);
}

/// Replaces the existing element with id `id`, taking ownership and extending the
/// new element's lifetime.
pub fn set_root_replace(id: &str, el: El) {
    document().get_element_by_id(id).unwrap().replace_with_with_node_1(&el.0.borrow().el).unwrap_throw();
    ROOT.with(|r| r.set(vec![el]));
}

/// Sets the elements as the children of the body, taking ownership and their
/// lifetimes.
pub fn set_root(elements: Vec<El>) {
    document()
        .body()
        .unwrap()
        .replace_children_with_node(&elements.iter().map(|e| e.0.borrow().el.clone()).collect());
    ROOT.with(|r| r.set(elements));
}
