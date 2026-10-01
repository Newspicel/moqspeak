//! A value that passes once through a command that has to be `Clone` and `Debug`.

/// Hands a value that is neither `Clone` nor `Debug` through a command that has to be both.
pub struct Handoff<T>(std::sync::Arc<std::sync::Mutex<Option<T>>>);

impl<T> Clone for Handoff<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Handoff<T> {
    pub fn new(value: T) -> Self {
        Self(std::sync::Arc::new(std::sync::Mutex::new(Some(value))))
    }

    pub fn take(&self) -> Option<T> {
        self.0.lock().unwrap().take()
    }
}

impl<T> std::fmt::Debug for Handoff<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Handoff(..)")
    }
}
