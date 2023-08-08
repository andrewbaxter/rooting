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
use wasm_bindgen::UnwrapThrowExt;
use web_sys::{
    Element,
    Node,
    Event,
    Document,
};

struct _ScopeValue<T>(T);

trait ScopeValue { }

impl<T> ScopeValue for _ScopeValue<T> { }

struct _ScopeEl {
    el: Element,
    parent: Option<Weak<RefCell<_ScopeEl>>>,
    index_in_parent: usize,
    children: Vec<ScopeElement>,
    local: Vec<Box<dyn ScopeValue>>,
}

impl _ScopeEl {
    fn _splice(&mut self, self2: &Rc<RefCell<_ScopeEl>>, offset: usize, remove: usize, add: Vec<ScopeElement>) {
        let el_children = self.el.children();

        // Remove existing dom children
        {
            for _ in 0 .. remove {
                el_children.get_with_index(offset as u32).unwrap_throw().remove();
            }
        }

        // Add new dom children + update parent state for new scope children
        let insert_ref = el_children.get_with_index(offset as u32);
        let insert_ref = insert_ref.as_ref().map(|x| x as &Node);
        for (i, child) in add.iter().enumerate() {
            let mut c = child.0.borrow_mut();
            if c.parent.is_some() {
                panic!("Adding child already in tree");
            }
            c.parent = Some(Rc::downgrade(self2));
            c.index_in_parent = offset + i;
            self.el.insert_before(&c.el, insert_ref).unwrap_throw();
        }

        // Splice scope children
        let count = add.len();
        self.children.splice(offset .. offset + remove, add);

        // Update parent state for old children after new children
        for i in offset + count .. self.children.len() {
            self.children[i].0.borrow_mut().index_in_parent = i;
        }
    }
}

/// An html element with associated data sharing the same lifetime. There are a
/// number of `init_` methods - these are the same as the non-init methods, but
/// consume `self` for easy chaining.  For modifying existing elements
/// post-creation use the non-`init_` methods.
///
/// `ScopeElement` values are clonable. Note that if you store a parent element in
/// a child element you'll end up with a reference cycle and the subtree will never
/// be freed.  You can use `weak()` to get a weak reference if you want to do this.
#[derive(Clone)]
pub struct ScopeElement(Rc<RefCell<_ScopeEl>>);

impl ScopeElement {
    /// Set text contents.
    pub fn init_text(self, text: &str) -> Self {
        self.0.borrow().el.set_text_content(Some(text));
        return self;
    }

    /// Set text contents.
    pub fn text(&self, text: &str) -> &Self {
        self.0.borrow().el.set_text_content(Some(text));
        return self;
    }

    pub fn init_id(self, id: &str) -> Self {
        self.0.borrow().el.set_id(id);
        return self;
    }

    pub fn init_attr(self, key: &str, value: &str) -> Self {
        self.0.borrow().el.set_attribute(key, value).unwrap_throw();
        return self;
    }

    pub fn attr(&self, key: &str, value: &str) -> &Self {
        self.0.borrow().el.set_attribute(key, value).unwrap_throw();
        return self;
    }

    pub fn init_classes(self, keys: &[&str]) -> Self {
        let c = self.0.borrow().el.class_list();
        for k in keys {
            c.add_1(k).unwrap();
        }
        return self;
    }

    pub fn classes(&self, keys: &[&str]) -> &Self {
        let c = self.0.borrow().el.class_list();
        for k in keys {
            c.add_1(k).unwrap();
        }
        return self;
    }

    pub fn remove_classes(&self, keys: &[&str]) -> &Self {
        let c = self.0.borrow().el.class_list();
        for k in keys {
            c.add_1(k).unwrap();
        }
        return self;
    }

    /// Add a single element to the end.
    pub fn init_append1(self, add: ScopeElement) -> Self {
        let len = self.0.borrow().children.len();
        self.splice(len, 0, vec![add]);
        return self;
    }

    /// Add a single element to the end.
    pub fn append1(&self, add: ScopeElement) -> &Self {
        let len = self.0.borrow().children.len();
        return self.splice(len, 0, vec![add]);
    }

    /// Add multiple elements to the end.
    pub fn init_append(self, add: Vec<ScopeElement>) -> Self {
        let mut s = self.0.borrow_mut();
        let len = s.children.len();
        s._splice(&self.0, len, 0, add);
        drop(s);
        return self;
    }

    /// Add multiple elements to the end.
    pub fn append(&self, add: Vec<ScopeElement>) -> &Self {
        let mut s = self.0.borrow_mut();
        let len = s.children.len();
        s._splice(&self.0, len, 0, add);
        drop(s);
        return self;
    }

    /// Add and remove multiple elements.
    pub fn splice(&self, offset: usize, remove: usize, add: Vec<ScopeElement>) -> &Self {
        self.0.borrow_mut()._splice(&self.0, offset, remove, add);
        return self;
    }

    /// Attach the value to this scope, so it doesn't get dropped until the element is
    /// removed from the tree.
    pub fn init_drop<T: 'static>(self, local: T) -> Self {
        self.0.borrow_mut().local.push(Box::new(_ScopeValue(local)));
        return self;
    }

    /// Attach the value to this scope, so it doesn't get dropped until the element is
    /// removed from the tree.
    pub fn drop<T: 'static>(&self, local: T) -> &Self {
        self.0.borrow_mut().local.push(Box::new(_ScopeValue(local)));
        return self;
    }

    /// Add a listener for an event. The listener will be detached when this element is
    /// dropped (removed from the tree).
    pub fn init_listen(self, event: &'static str, cb: impl FnMut(&Event) + 'static) -> Self {
        let mut s = self.0.borrow_mut();
        let listener = EventListener::new(&s.el, event, cb);
        s.local.push(Box::new(_ScopeValue(listener)));
        drop(s);
        return self;
    }

    /// Add a listener for an event. The listener will be detached when this element is
    /// dropped (removed from the tree).
    pub fn listen(&self, event: &'static str, cb: impl FnMut(&Event) + 'static) -> &Self {
        let mut s = self.0.borrow_mut();
        let listener = EventListener::new(&s.el, event, cb);
        s.local.push(Box::new(_ScopeValue(listener)));
        drop(s);
        return self;
    }

    /// Produce a weak reference to the element.
    pub fn weak(&self) -> WeakScopeElement {
        return WeakScopeElement(Rc::downgrade(&self.0));
    }
}

#[derive(Clone)]
pub struct WeakScopeElement(Weak<RefCell<_ScopeEl>>);

impl WeakScopeElement {
    pub fn upgrade(&self) -> Option<ScopeElement> {
        return Some(ScopeElement(self.0.upgrade()?));
    }
}

/// Helper to get the document.
pub fn doc() -> Document {
    return web_sys::window().unwrap_throw().document().unwrap_throw();
}

/// Create a new element.
pub fn el(tag: &str) -> ScopeElement {
    return ScopeElement(Rc::new(RefCell::new(_ScopeEl {
        el: doc().create_element(tag).unwrap(),
        parent: None,
        index_in_parent: 0,
        children: vec![],
        local: vec![],
    })));
}

thread_local!{
    static ROOT: Cell<Option<ScopeElement>> = Cell::new(None);
}

/// Set the element as the root, replacing the existing element with id `id`.
pub fn set_root_replace(id: &str, el: ScopeElement) {
    doc().get_element_by_id(id).unwrap_throw().replace_with_with_node_1(&el.0.borrow().el).unwrap_throw();
    ROOT.with(|r| r.set(Some(el)));
}

/// Set the element as the root, adding the element to the document body.
pub fn set_root(el: ScopeElement) {
    doc().body().unwrap_throw().append_child(&el.0.borrow().el).unwrap_throw();
    ROOT.with(|r| r.set(Some(el)));
}
