use crate::types::{Structured, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct DeviceState {
    inner: Arc<Mutex<Structured>>,
}

impl Default for DeviceState {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceState {
    #[must_use]
    pub fn new() -> Self {
        DeviceState {
            inner: Arc::new(Mutex::new(Structured {
                name: String::new(),
                properties: HashMap::default(),
                children: vec![],
            })),
        }
    }

    fn lock(&self) -> crate::Result<std::sync::MutexGuard<'_, Structured>> {
        self.inner
            .lock()
            .map_err(|e| crate::Error::State(format!("state lock poisoned: {e}")))
    }

    /// Replaces the entire device state tree.
    ///
    /// # Errors
    /// Returns an error if the state lock is poisoned.
    pub fn replace(&self, state: Structured) -> crate::Result<()> {
        let mut guard = self.lock()?;
        *guard = state;
        Ok(())
    }

    /// Returns a clone of the current state tree.
    ///
    /// # Errors
    /// Returns an error if the state lock is poisoned.
    pub fn snapshot(&self) -> crate::Result<Structured> {
        self.lock().map(|guard| guard.clone())
    }

    /// Sets a property value at the given path in the state tree.
    ///
    /// # Errors
    /// Returns an error if the lock is poisoned, the path is invalid, or types mismatch.
    pub fn set_property(
        &self,
        indices: &[usize],
        property_name: &str,
        value: Value,
    ) -> crate::Result<()> {
        let mut guard = self.lock()?;
        guard.set_property(indices, property_name, value)
    }

    /// Returns whether the state tree has been populated.
    ///
    /// # Errors
    /// Returns an error if the state lock is poisoned.
    pub fn is_initialized(&self) -> crate::Result<bool> {
        let guard = self.lock()?;
        Ok(!guard.name.is_empty())
    }
}
