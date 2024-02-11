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

struct Defer<F: 'static + FnOnce() -> ()>(Option<F>);

impl<F: 'static + FnOnce() -> ()> Drop for Defer<F> {
    fn drop(&mut self) {
        (self.0.take().unwrap())();
    }
}

pub fn defer<F: 'static + FnOnce() -> ()>(f: F) -> ScopeValue {
    return scope_any(Defer(Some(f)));
}
