This is a library for managing lifetimes of data associated with DOM elements in WASM. This is for an a la carte UI solution, rather than using a framework which would typically take care of this for you.

This manages a globally owned tree of `ScopeElement` which is proxied onto the actual DOM. Attach data to a `ScopedElement` with the `e.drop(data)` method. The data will be destroyed when the element is (when removed from the tree, if no other references exist).

Create and modify `ScopeElement`s instead of using `create_element` and doing direct modification.

Trivial example:

```
#[wasm_bindgen(start)]
pub fn main() {
    set_root(el("button").init_text("Click me baby").init_listen("click", |_| console_dbg!("Clicked")));
}
```

In this example, the callback lifetime is bound to the button's lifetime, and will be detached when the button is removed from the root (although that never happens in this example).
