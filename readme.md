This is a library for managing lifetimes of data associated with DOM elements in WASM. This is useful because `wasm-bindgen`/`web_sys` tie lifetimes to Rust scope rather than JS garbage collection - but even if they did (with new browser GC features) tying state to elements like this produces tightly bound lifetimes which, depending on your use case, can make reasoning about state simpler when using GC.

This is for an a la carte UI solution, rather than using a framework which would typically take care of this for you.

The core object in this library is `El` which represents a DOM element and the lifetime-associated data. `El` acts as a replacement for the corresponding `web_sys` element types, with most common methods for mutation exposed. If you need to access the underlying element, you can do so with `.raw()`.

Attach data to an `El` with the `e.own(|e| some_data)` method. The returned data will be dropped when the element is (when removed from the tree, if no other references exist).

# Example

```
pub fn main() {
    set_root(vec![
        el("button").text("Click me baby").on("click", |_| console_dbg!("Clicked"))
    ]);
}
```

In this example, the callback lifetime is bound to the button's lifetime, and will be detached when the button is removed from the root (although that never happens in this example).

# Links

- [rooting-form](https://github.com/andrewbaxter/rooting-form) - derive macro to generate a form from a struct, with validation and error messages.
