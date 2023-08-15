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
use wasm_bindgen::UnwrapThrowExt;
use web_sys::{
    Element,
    Node,
    Event,
};

struct _ScopeValue<T>(T);

pub trait ScopeValue { }

impl<T> ScopeValue for _ScopeValue<T> { }

pub fn scope_any<T: 'static>(value: T) -> Box<dyn ScopeValue> {
    return Box::new(_ScopeValue(value));
}

struct _ScopeElement {
    el: Element,
    parent: Option<Weak<RefCell<_ScopeElement>>>,
    index_in_parent: usize,
    children: Vec<ScopeElement>,
    local: Vec<Box<dyn ScopeValue>>,
}

impl _ScopeElement {
    fn splice(
        &mut self,
        self2: &Rc<RefCell<_ScopeElement>>,
        offset: usize,
        remove: usize,
        add: Vec<ScopeElement>,
    ) {
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

    fn clear(&mut self) {
        self.el.set_text_content(None);
        self.children.clear();
    }

    fn extend(&mut self, self2: &Rc<RefCell<_ScopeElement>>, add: Vec<ScopeElement>) {
        let offset = self.children.len();
        for (i, child) in add.iter().enumerate() {
            let mut c = child.0.borrow_mut();
            c.parent = Some(Rc::downgrade(self2));
            c.index_in_parent = offset + i;
            self.el.append_child(&c.el).unwrap_throw();
        }
        self.children.extend(add);
    }
}

/// An html element with associated data sharing the same lifetime. There are a
/// number of `mut_` and non-`mut_` method pairs. The non-`mut_` methods are
/// chainable, for use during element creation. The `mut_` methods are for
/// modifying existing elements.
///
/// `ScopeElement` values are clonable. Note that if you store a parent element in
/// a child element you'll end up with a reference cycle and the subtree will never
/// be freed.  You can use `weak()` to get a weak reference if you want to do this.
#[derive(Clone)]
pub struct ScopeElement(Rc<RefCell<_ScopeElement>>);

impl ScopeElement {
    pub fn text(self, text: &str) -> Self {
        self.0.borrow().el.set_text_content(Some(text));
        return self;
    }

    /// Set text contents.
    pub fn mut_text(&self, text: &str) -> &Self {
        self.0.borrow().el.set_text_content(Some(text));
        return self;
    }

    /// Set the element id.
    pub fn id(self, id: &str) -> Self {
        self.0.borrow().el.set_id(id);
        return self;
    }

    pub fn attr(self, key: &str, value: &str) -> Self {
        self.0.borrow().el.set_attribute(key, value).unwrap_throw();
        return self;
    }

    /// Set an arbitrary attribute.  Note there are special methods for setting `class`
    /// and `id` which may afford safer workflows.
    pub fn mut_attr(&self, key: &str, value: &str) -> &Self {
        self.0.borrow().el.set_attribute(key, value).unwrap_throw();
        return self;
    }

    pub fn classes(self, keys: &[&str]) -> Self {
        let c = self.0.borrow().el.class_list();
        for k in keys {
            c.add_1(k).unwrap();
        }
        return self;
    }

    /// Add (if not existing) all of the listed keys.
    pub fn mut_classes(&self, keys: &[&str]) -> &Self {
        let c = self.0.borrow().el.class_list();
        for k in keys {
            c.add_1(k).unwrap();
        }
        return self;
    }

    /// Remove (if not existing) all of the listed keys.
    pub fn mut_remove_classes(&self, keys: &[&str]) -> &Self {
        let c = self.0.borrow().el.class_list();
        for k in keys {
            c.add_1(k).unwrap();
        }
        return self;
    }

    pub fn push(self, add: ScopeElement) -> Self {
        self.0.borrow_mut().extend(&self.0, vec![add]);
        return self;
    }

    /// Add a single element to the end.
    pub fn mut_push(&self, add: ScopeElement) -> &Self {
        self.0.borrow_mut().extend(&self.0, vec![add]);
        return self;
    }

    pub fn extend(self, add: Vec<ScopeElement>) -> Self {
        self.0.borrow_mut().extend(&self.0, add);
        return self;
    }

    /// Add multiple elements to the end.
    pub fn mut_extend(&self, add: Vec<ScopeElement>) -> &Self {
        self.0.borrow_mut().extend(&self.0, add);
        return self;
    }

    /// Add and remove multiple elements.
    pub fn mut_splice(&self, offset: usize, remove: usize, add: Vec<ScopeElement>) -> &Self {
        self.0.borrow_mut().splice(&self.0, offset, remove, add);
        return self;
    }

    /// Remove all children.
    pub fn mut_clear(&self) -> &Self {
        self.0.borrow_mut().clear();
        return self;
    }

    pub fn drop<T: 'static>(self, supplier: impl FnOnce(&ScopeElement) -> T) -> Self {
        self.0.borrow_mut().local.push(Box::new(_ScopeValue(supplier(&self))));
        return self;
    }

    /// Attach the value to this scope, so it doesn't get dropped until the element is
    /// removed from the tree.
    pub fn mut_drop<T: 'static>(&self, local: T) -> &Self {
        self.0.borrow_mut().local.push(Box::new(_ScopeValue(local)));
        return self;
    }

    pub fn on(self, event: &'static str, cb: impl FnMut(&Event) + 'static) -> Self {
        let mut s = self.0.borrow_mut();
        let listener = EventListener::new(&s.el, event, cb);
        s.local.push(Box::new(_ScopeValue(listener)));
        drop(s);
        return self;
    }

    /// Add a listener for an event. The listener will be detached when this element is
    /// dropped (removed from the tree).
    pub fn mut_listen(&self, event: &'static str, cb: impl FnMut(&Event) + 'static) -> &Self {
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
pub struct WeakScopeElement(Weak<RefCell<_ScopeElement>>);

impl WeakScopeElement {
    pub fn upgrade(&self) -> Option<ScopeElement> {
        return Some(ScopeElement(self.0.upgrade()?));
    }
}

/// Create a new element.
pub fn el(tag: &str) -> ScopeElement {
    return ScopeElement(Rc::new(RefCell::new(_ScopeElement {
        el: document().create_element(tag).unwrap(),
        parent: None,
        index_in_parent: 0,
        children: vec![],
        local: vec![],
    })));
}

/// Create a new scoped element from an element passed in (ex: for existing
/// elements, or namespaced elements set up specially).
pub fn el_from_raw(el: Element) -> ScopeElement {
    return ScopeElement(Rc::new(RefCell::new(_ScopeElement {
        el: el,
        parent: None,
        index_in_parent: 0,
        children: vec![],
        local: vec![],
    })));
}

thread_local!{
    static ROOT: Cell<Vec<ScopeElement>> = Cell::new(vec![]);
}

/// Replaces the existing element with id `id`, taking ownership and extending the
/// new element's lifetime.
pub fn set_root_replace(id: &str, el: ScopeElement) {
    document().get_element_by_id(id).unwrap_throw().replace_with_with_node_1(&el.0.borrow().el).unwrap_throw();
    ROOT.with(|r| r.set(vec![el]));
}

/// Sets the elements as the children of the body, taking ownership and their
/// lifetimes.
pub fn set_root(elements: Vec<ScopeElement>) {
    document()
        .body()
        .unwrap_throw()
        .replace_children_with_node(&elements.iter().map(|e| e.0.borrow().el.clone()).collect());
    ROOT.with(|r| r.set(elements));
}
