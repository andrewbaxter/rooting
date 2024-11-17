use std::{
    rc::{
        Weak,
        Rc,
    },
};
use js_sys::Array;
use wasm_bindgen::{
    prelude::Closure,
    JsValue,
    JsCast,
};
use web_sys::{
    Element,
    ResizeObserver as ResizeObserver1,
    ResizeObserverOptions,
};
use crate::{
    scope_any,
    ScopeValue,
};

pub struct ResizeObserver_ {
    pub js_resize_observer: ResizeObserver1,
    _js_cb: ScopeValue,
}

/// This is a convenience wrapper around `web_sys` `ResizeObserver`, used within
/// `El` methods but also usable externally.  Per the ECMAScript design
/// discussions, if you need to monitor multiple elements with the same callback,
/// using a single `ResizeObserver` is faster than using multiple `ResizeObserver`s.
#[derive(Clone)]
pub struct ResizeObserver(pub Rc<ResizeObserver_>);

/// See `ResizeObserver`'s `observe` method.
pub struct ObserveHandle {
    target: Element,
    resize_observer: Weak<ResizeObserver_>,
}

impl ResizeObserver {
    /// The callback receives an array with all observed elements that changed size
    /// this tick. You can retrieve the new sizes like:
    ///
    /// ```
    /// let entry = entries.get(0).dyn_into::<ResizeObserverEntry>().unwrap();
    /// let size = entry.content_box_size().get(0).dyn_into::<ResizeObserverSize>().unwrap();
    /// (size.inline_size(), size.block_size())
    /// ```
    pub fn new(cb: impl Fn(Array) + 'static) -> Self {
        let js_cb = Closure::wrap(Box::new(move |entries: Array, _| -> () {
            cb(entries);
        }) as Box<dyn Fn(Array, JsValue)>);
        let resize_observer = ResizeObserver1::new(js_cb.as_ref().unchecked_ref()).unwrap();
        return Self(Rc::new(ResizeObserver_ {
            js_resize_observer: resize_observer,
            _js_cb: scope_any(js_cb),
        }));
    }

    /// Add the target element to the observation set - if the element changes size the
    /// callback will be invoked.  The callback will also be invoked immediately (with
    /// the current stack) for the specified element when this method is called.
    ///
    /// When the `ObserveHandle` is dropped, the target will stop being observed.
    pub fn observe(&self, target: &Element) -> ObserveHandle {
        self.0.js_resize_observer.observe(target.dyn_ref().unwrap());
        return ObserveHandle {
            target: target.clone(),
            resize_observer: Rc::downgrade(&self.0),
        }
    }

    pub fn observe_with_options(&self, target: &Element, opts: &ResizeObserverOptions) -> ObserveHandle {
        self.0.js_resize_observer.observe_with_options(target.dyn_ref().unwrap(), opts);
        return ObserveHandle {
            target: target.clone(),
            resize_observer: Rc::downgrade(&self.0),
        }
    }
}

impl Drop for ObserveHandle {
    fn drop(&mut self) {
        let Some(resize_observer) = self.resize_observer.upgrade() else {
            return;
        };
        resize_observer.js_resize_observer.unobserve(self.target.dyn_ref().unwrap());
    }
}
