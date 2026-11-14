//! Small helpers for widget callbacks.

use std::cell::RefCell;

/// A replaceable callback taking one argument.
pub struct Slot<T>(RefCell<Option<Callback<T>>>);

type Callback<T> = Box<dyn Fn(T)>;

impl<T> Default for Slot<T> {
    fn default() -> Self {
        Slot(RefCell::new(None))
    }
}

impl<T> Slot<T> {
    pub fn connect(&self, f: impl Fn(T) + 'static) {
        *self.0.borrow_mut() = Some(Box::new(f));
    }

    pub fn emit(&self, value: T) {
        if let Some(f) = self.0.borrow().as_ref() {
            f(value);
        }
    }
}
