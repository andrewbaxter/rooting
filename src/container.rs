use crate::El;

/// A trait describing data structures that have a representative `El`.  This is
/// for use with `Container`.
pub trait ContainerEntry {
    fn el(&self) -> &El;
}

/// This pairs a vec with an El, so when you modify the vec the changes are
/// mirrored to the element.  Any type that implements `ContainerEntry` can be used.
///
/// Warning: If you use `ref_remove()` to remove an element via the child, or make
/// changes via `.el()`, the lists will get out of sync and behavior is not
/// guaranteed.  (If you add an element via `.el()` then remove it before calling
/// further methods it should be fine.)
pub struct Container<T: ContainerEntry> {
    entries: Vec<T>,
    el: El,
}

impl<T: ContainerEntry> Container<T> {
    pub fn new(el: El) -> Container<T> {
        return Container {
            entries: vec![],
            el: el,
        };
    }

    pub fn iter(&self) -> core::slice::Iter<T> {
        return self.entries.iter();
    }

    pub fn clear(&mut self) {
        self.el.ref_clear();
        self.entries.clear();
    }

    pub fn push(&mut self, entry: T) {
        self.el.ref_push(entry.el().clone());
        self.entries.push(entry);
    }

    pub fn extend(&mut self, entries: Vec<T>) {
        self.el.ref_extend(entries.iter().map(|e| e.el().clone()).collect());
        self.entries.extend(entries);
    }

    pub fn insert(&mut self, i: usize, entry: T) {
        self.el.ref_splice(i, 0, vec![entry.el().clone()]);
        self.entries.insert(i, entry);
    }

    pub fn splice(
        &mut self,
        offset: usize,
        remove: usize,
        add: Vec<T>,
    ) -> std::vec::Splice<'_, std::vec::IntoIter<T>> {
        self.el.ref_splice(offset, remove, add.iter().map(|e| e.el().clone()).collect());
        return self.entries.splice(offset .. offset + remove, add);
    }

    pub fn first(&self) -> Option<&T> {
        return self.entries.first();
    }

    pub fn last(&self) -> Option<&T> {
        return self.entries.last();
    }

    pub fn first_mut(&mut self) -> Option<&mut T> {
        return self.entries.first_mut();
    }

    pub fn last_mut(&mut self) -> Option<&mut T> {
        return self.entries.last_mut();
    }

    pub fn pop(&mut self) -> Option<T> {
        let len = self.entries.len();
        if len > 0 {
            self.el.ref_splice(len - 1, 1, vec![]);
            return self.entries.pop();
        } else {
            return None;
        }
    }

    pub fn len(&self) -> usize {
        return self.entries.len();
    }

    pub fn is_empty(&self) -> bool {
        return self.entries.is_empty();
    }

    pub fn is_some(&self) -> bool {
        return !self.entries.is_empty();
    }

    pub fn get(&self, i: usize) -> Option<&T> {
        return self.entries.get(i);
    }

    pub fn remove(&mut self, i: usize) -> T {
        self.el.ref_splice(i, 1, vec![]);
        return self.entries.remove(i);
    }
}

impl<T: ContainerEntry> ContainerEntry for Container<T> {
    fn el(&self) -> &El {
        return &self.el;
    }
}

impl<'a, T: ContainerEntry> IntoIterator for &'a Container<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        return (&self.entries).into_iter();
    }
}
