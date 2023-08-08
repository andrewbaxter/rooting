This is a library for managing lifetimes of data associated with DOM elements in WASM.

This manages a globally owned tree of `ScopeElement` which is proxied onto the actual DOM. `ScopedElement` can have attached data via the `e.drop(data)` method, which will be destroyed when the element is dropped (removed from the tree, if no other references exist).

Trivial example:

```
#[wasm_bindgen(start)]
pub fn main() {
    set_root(el("button").init_text("Click me baby").init_listen("click", |_| console_dbg!("Clicked")));
}
```