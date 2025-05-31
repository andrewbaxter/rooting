use {
    std::{
        rc::{
            Weak,
            Rc,
        },
        cell::{
            RefCell,
        },
    },
    gloo_events::{
        EventListener,
        EventListenerOptions,
    },
    gloo_utils::document,
    wasm_bindgen::{
        JsCast,
        JsValue,
    },
    web_sys::{
        Element,
        Node,
        Event,
        ResizeObserverEntry,
        ResizeObserverSize,
    },
    crate::{
        own::{
            scope_any,
            ScopeValue,
        },
        resize::{
            ResizeObserver,
        },
    },
};

pub(crate) struct El_ {
    pub(crate) el: Element,
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
pub struct El(pub(crate) Rc<RefCell<El_>>);

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

    pub fn ref_own<T: 'static>(&self, supplier: impl FnOnce(&El) -> T) -> &Self {
        let res = supplier(&self);
        self.0.borrow_mut().local.push(scope_any(res));
        return self;
    }

    pub fn on(self, event: &'static str, cb: impl FnMut(&Event) + 'static) -> Self {
        self.ref_on(event, cb);
        return self;
    }

    pub fn on_with_options(
        self,
        event: &'static str,
        opts: EventListenerOptions,
        cb: impl FnMut(&Event) + 'static,
    ) -> Self {
        self.ref_on_with_options(event, opts, cb);
        return self;
    }

    pub fn ref_on(&self, event: &'static str, cb: impl FnMut(&Event) + 'static) -> &Self {
        let mut s = self.0.borrow_mut();
        let listener = EventListener::new(&s.el, event, cb);
        s.local.push(scope_any(listener));
        drop(s);
        return self;
    }

    pub fn ref_on_with_options(
        &self,
        event: &'static str,
        opts: EventListenerOptions,
        cb: impl FnMut(&Event) + 'static,
    ) -> &Self {
        let mut s = self.0.borrow_mut();
        let listener = EventListener::new_with_options(&s.el, event, opts, cb);
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
            let handle = resize_observer.observe(&e.raw());
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

    /// Replace the element in its parent with zero or more new elements.
    pub fn ref_replace(&self, other: Vec<El>) {
        let mut self1 = self.0.borrow_mut();
        if let Some(el_parent) = self1.parent.as_ref().and_then(|x| x.upgrade()) {
            // Proper replacement -- this element is removed from its parent and the
            // replacement is put in its place. This El can be reused wherever, but obviously
            // without re-placing it further modifications won't appear anywhere.
            let index_in_parent = self1.index_in_parent;
            drop(self1);
            El(el_parent).ref_splice(index_in_parent, 1, other);
        } else {
            // Pseudo-replacement, for when you have disconnected hierarchies (objects with
            // multiple El in a DOM tree but no El parent-child relation)
            //
            // This element is hollowed out and the replacement is stored as an "owned value".
            // You shouldn't reuse this element directly, it now exists just to root the
            // replacements. Modifications won't fail, but won't appear anywhere, and
            // owning/adding event listeners will just waste memory.
            self1.children.clear();
            self1.local.clear();
            self1.local.push(scope_any(other.clone()));
            self1
                .el
                .replace_with_with_node(&&other.into_iter().map(|x| JsValue::from(x.raw())).collect())
                .expect("Failed to replace element with new elements");
            self1.el = document().create_element("div").unwrap();
        }
    }

    /// Get the wrapped web_sys element from the El.
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
