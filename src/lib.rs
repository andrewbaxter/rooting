use std::{
    rc::{
        Weak,
        Rc,
    },
    cell::{
        RefCell,
        Cell,
    },
};
use gloo_events::EventListener;
use gloo_utils::document;
use js_sys::Array;
use wasm_bindgen::{
    UnwrapThrowExt,
    prelude::Closure,
    JsValue,
    JsCast,
};
use web_sys::{
    Element,
    Node,
    Event,
    ResizeObserverEntry,
    ResizeObserver as ResizeObserver1,
    ResizeObserverSize,
};

struct ScopeValue_<T>(T);

pub trait ScopeValueTrait_ { }

impl<T> ScopeValueTrait_ for ScopeValue_<T> { }

/// This is a wrapper type that can hold any object opaquely (see `scope_any`) and
/// will execute `Drop` for the contained object.
pub struct ScopeValue(Box<dyn ScopeValueTrait_>);

/// This converts anything into a single type, so you can put it in a collection.
/// The primary use for this is storing guard/drop values which don't do anything
/// while alive, but execute some code when dropped.  This is used by `.own(...)`
/// below to store arbitrary data in the `El`.
pub fn scope_any<T: 'static>(value: T) -> ScopeValue {
    return ScopeValue(Box::new(ScopeValue_(value)));
}

struct ResizeObserver_ {
    resize_observer: ResizeObserver1,
    _js_cb: ScopeValue,
}

/// This is a convenience wrapper around `web_sys` `ResizeObserver`, used within
/// `El` methods but also usable externally.  Per the ECMAScript design
/// discussions, if you need to monitor multiple elements with the same callback,
/// using a single `ResizeObserver` is faster than using multiple `ResizeObserver`s.
#[derive(Clone)]
pub struct ResizeObserver(Rc<ResizeObserver_>);

/// See `ResizeObserver`'s `observe` method.
pub struct ObserveHandle {
    target: WeakEl,
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
            resize_observer: resize_observer,
            _js_cb: scope_any(js_cb),
        }));
    }

    /// Add the target element to the observation set - if the element changes size the
    /// callback will be invoked.  The callback will also be invoked immediately (with
    /// the current stack) for the specified element when this method is called.
    ///
    /// When the `ObserveHandle` is dropped, the target will stop being observed.
    pub fn observe(&self, target: &El) -> ObserveHandle {
        self.0.resize_observer.observe(target.raw().dyn_ref().unwrap());
        return ObserveHandle {
            target: target.weak(),
            resize_observer: Rc::downgrade(&self.0),
        }
    }
}

impl Drop for ObserveHandle {
    fn drop(&mut self) {
        let Some(resize_observer) = self.resize_observer.upgrade() else {
            return;
        };
        let Some(target) = self.target.upgrade() else {
            return;
        };
        resize_observer.resize_observer.unobserve(target.raw().dyn_ref().unwrap());
    }
}

struct El_ {
    el: Element,
    parent: Option<Weak<RefCell<El_>>>,
    index_in_parent: usize,
    children: Vec<El>,
    local: Vec<ScopeValue>,
}

impl El_ {
    fn splice(&mut self, self2: &Rc<RefCell<El_>>, offset: usize, remove: usize, add: Vec<El>) {
        let el_children = self.el.children();

        // Remove existing dom children
        for _ in 0 .. remove {
            el_children.get_with_index(offset as u32).unwrap().remove();
        }

        // Add new dom children + update parent state for new scope children
        let insert_ref = el_children.get_with_index(offset as u32);
        let insert_ref = insert_ref.as_ref().map(|x| x as &Node);
        for (i, child) in add.iter().enumerate() {
            let mut c = child.0.borrow_mut();
            c.parent = Some(Rc::downgrade(self2));
            c.index_in_parent = offset + i;
            self.el.insert_before(&c.el, insert_ref).unwrap();
        }

        // Splice scope children
        let count = add.len();
        let removed = self.children.splice(offset .. offset + remove, add);

        // Clear parent state for removed children
        for child in removed {
            let mut child = child.0.borrow_mut();
            child.parent = None;
            child.index_in_parent = 0;
        }

        // Update parent state for old children after new children
        for i in offset + count .. self.children.len() {
            self.children[i].0.borrow_mut().index_in_parent = i;
        }
    }

    fn clear(&mut self) {
        self.el.set_text_content(None);
        self.children.clear();
    }

    fn extend(&mut self, self2: &Rc<RefCell<El_>>, add: Vec<El>) {
        let offset = self.children.len();
        for (i, child) in add.iter().enumerate() {
            let mut c = child.0.borrow_mut();
            c.parent = Some(Rc::downgrade(self2));
            c.index_in_parent = offset + i;
            self.el.append_child(&c.el).unwrap();
        }
        self.children.extend(add);
    }
}

/// An html element with associated data sharing the same lifetime.
///
/// There are a number of `ref_` and non-`ref_` method pairs. The non-`ref_`
/// methods are chainable but consume and return the element, for use during
/// element creation. The `ref_` take a reference and as such don't consume or
/// return the element.
///
/// `El` values are clonable. Note that if you store a parent element in a child
/// element you'll end up with a reference cycle and the subtree will never be
/// freed.  You can use `weak()` to get a weak reference if you want to do this.
#[derive(Clone)]
pub struct El(Rc<RefCell<El_>>);

impl El {
    /// Set text contents.
    pub fn text(self, text: &str) -> Self {
        self.0.borrow().el.set_text_content(Some(text));
        return self;
    }

    pub fn ref_text(&self, text: &str) -> &Self {
        self.0.borrow().el.set_text_content(Some(text));
        return self;
    }

    /// Set the element id.
    pub fn id(self, id: &str) -> Self {
        self.0.borrow().el.set_id(id);
        return self;
    }

    /// Set an arbitrary attribute.  Note there are special methods for setting `class`
    /// and `id` which may afford safer workflows.
    pub fn attr(self, key: &str, value: &str) -> Self {
        self.0.borrow().el.set_attribute(key, value).unwrap();
        return self;
    }

    pub fn ref_attr(&self, key: &str, value: &str) -> &Self {
        self.0.borrow().el.set_attribute(key, value).unwrap();
        return self;
    }

    /// Remove an attribute from the element.
    pub fn ref_remove_attr(&self, key: &str) -> &Self {
        self.0.borrow().el.remove_attribute(key).unwrap();
        return self;
    }

    /// Add (if not existing) all of the listed keys.
    pub fn classes(self, keys: &[&str]) -> Self {
        let c = self.0.borrow().el.class_list();
        for k in keys {
            c.add_1(k).unwrap();
        }
        return self;
    }

    pub fn ref_classes(&self, keys: &[&str]) -> &Self {
        let c = self.0.borrow().el.class_list();
        for k in keys {
            c.add_1(k).unwrap();
        }
        return self;
    }

    /// Remove (if not existing) all of the listed keys.
    pub fn ref_remove_classes(&self, keys: &[&str]) -> &Self {
        let c = self.0.borrow().el.class_list();
        for k in keys {
            c.remove_1(k).unwrap();
        }
        return self;
    }

    pub fn ref_modify_classes(&self, keys: &[(&str, bool)]) -> &Self {
        let c = self.0.borrow().el.class_list();
        for (k, on) in keys {
            if *on {
                c.add_1(k).unwrap();
            } else {
                c.remove_1(k).unwrap();
            }
        }
        return self;
    }

    /// Add a single element to the end.
    pub fn push(self, add: El) -> Self {
        self.0.borrow_mut().extend(&self.0, vec![add]);
        return self;
    }

    pub fn ref_push(&self, add: El) -> &Self {
        self.0.borrow_mut().extend(&self.0, vec![add]);
        return self;
    }

    /// Add multiple elements to the end.
    pub fn extend(self, add: Vec<El>) -> Self {
        self.0.borrow_mut().extend(&self.0, add);
        return self;
    }

    pub fn ref_extend(&self, add: Vec<El>) -> &Self {
        self.0.borrow_mut().extend(&self.0, add);
        return self;
    }

    /// Add and remove multiple elements.
    pub fn ref_splice(&self, offset: usize, remove: usize, add: Vec<El>) -> &Self {
        self.0.borrow_mut().splice(&self.0, offset, remove, add);
        return self;
    }

    /// Remove all children.
    pub fn ref_clear(&self) -> &Self {
        self.0.borrow_mut().clear();
        return self;
    }

    /// Attach the value to this scope, so it doesn't get dropped until the element is
    /// removed from the tree.
    pub fn own<T: 'static>(self, supplier: impl FnOnce(&El) -> T) -> Self {
        let res = supplier(&self);
        self.0.borrow_mut().local.push(scope_any(res));
        return self;
    }

    pub fn ref_own<T: 'static>(&self, local: T) -> &Self {
        self.0.borrow_mut().local.push(scope_any(local));
        return self;
    }

    pub fn on(self, event: &'static str, cb: impl FnMut(&Event) + 'static) -> Self {
        self.ref_on(event, cb);
        return self;
    }

    pub fn ref_on(&self, event: &'static str, cb: impl FnMut(&Event) + 'static) -> &Self {
        let mut s = self.0.borrow_mut();
        let listener = EventListener::new(&s.el, event, cb);
        s.local.push(scope_any(listener));
        drop(s);
        return self;
    }

    /// Adds a resize callback via `ResizeObserver`.  The callback is called on a
    /// resize with the element's first block's inline and block size as arguments
    /// (width and height for row layout, height and width for column layout).
    pub fn on_resize(self, cb: impl Fn(El, f64, f64) + 'static) -> Self {
        self.ref_on_resize(cb);
        return self;
    }

    pub fn ref_on_resize(&self, cb: impl Fn(El, f64, f64) + 'static) -> &Self {
        return self.ref_own(move |e: &El| {
            let resize_observer = ResizeObserver::new({
                let e = e.weak();
                move |entries| {
                    let Some(e) = e.upgrade() else {
                        return;
                    };
                    let entry: ResizeObserverEntry = entries.get(0).dyn_into::<ResizeObserverEntry>().unwrap();
                    let size = entry.content_box_size().get(0).dyn_into::<ResizeObserverSize>().unwrap();
                    cb(e, size.inline_size(), size.block_size());
                }
            });
            let handle = resize_observer.observe(e);
            return (resize_observer, handle);
        });
    }

    /// Add a listener for an event. The listener will be detached when this element is
    /// dropped (removed from the tree).
    pub fn ref_listen(&self, event: &'static str, cb: impl FnMut(&Event) + 'static) -> &Self {
        let mut s = self.0.borrow_mut();
        let listener = EventListener::new(&s.el, event, cb);
        s.local.push(scope_any(listener));
        drop(s);
        return self;
    }

    /// Remove the element from its parent.
    pub fn ref_remove(&self) {
        let parent;
        let index_in_parent;
        {
            let self1 = self.0.borrow();
            index_in_parent = self1.index_in_parent;
            let Some(parent1) = self1.parent.as_ref().and_then(|p| p.upgrade()) else {
                return;
            };
            parent = parent1;
        }
        El(parent).ref_splice(index_in_parent, 1, vec![]);
    }

    pub fn raw(&self) -> Element {
        return self.0.borrow().el.clone();
    }

    /// For debugging, an id based on pointer address
    pub fn ptr_id(&self) -> usize {
        return Rc::as_ptr(&self.0) as usize;
    }

    /// Produce a weak reference to the element.
    pub fn weak(&self) -> WeakEl {
        return WeakEl(Rc::downgrade(&self.0));
    }
}

#[derive(Clone)]
pub struct WeakEl(Weak<RefCell<El_>>);

impl WeakEl {
    pub fn upgrade(&self) -> Option<El> {
        return Some(El(self.0.upgrade()?));
    }
}

/// Create a new element.
pub fn el(tag: &str) -> El {
    return El(Rc::new(RefCell::new(El_ {
        el: document().create_element(tag).unwrap(),
        parent: None,
        index_in_parent: 0,
        children: vec![],
        local: vec![],
    })));
}

/// Create a new scoped element from an element passed in (ex: for existing
/// elements, or namespaced elements set up specially).
pub fn el_from_raw(el: Element) -> El {
    return El(Rc::new(RefCell::new(El_ {
        el: el,
        parent: None,
        index_in_parent: 0,
        children: vec![],
        local: vec![],
    })));
}

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
