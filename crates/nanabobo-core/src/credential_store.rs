use std::sync::Mutex;

use keyring::Entry;
use thiserror::Error;

const BILIBILI_ACCOUNT: &str = "bilibili";

#[derive(Debug, Error)]
pub enum CredentialStoreError {
    #[error("credential store unavailable")]
    Unavailable,
}

pub trait CredentialStore: Send + Sync {
    fn load(&self) -> Result<Option<String>, CredentialStoreError>;
    fn save(&self, value: &str) -> Result<(), CredentialStoreError>;
    fn clear(&self) -> Result<(), CredentialStoreError>;
}

pub struct KeyringCredentialStore {
    service: String,
}

impl KeyringCredentialStore {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    fn entry(&self) -> Result<Entry, CredentialStoreError> {
        Entry::new(&self.service, BILIBILI_ACCOUNT).map_err(|_| CredentialStoreError::Unavailable)
    }
}

impl CredentialStore for KeyringCredentialStore {
    fn load(&self) -> Result<Option<String>, CredentialStoreError> {
        match self.entry()?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(CredentialStoreError::Unavailable),
        }
    }

    fn save(&self, value: &str) -> Result<(), CredentialStoreError> {
        self.entry()?
            .set_password(value)
            .map_err(|_| CredentialStoreError::Unavailable)
    }

    fn clear(&self) -> Result<(), CredentialStoreError> {
        match self.entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(CredentialStoreError::Unavailable),
        }
    }
}

#[derive(Default)]
pub struct MemoryCredentialStore {
    value: Mutex<Option<String>>,
}

impl CredentialStore for MemoryCredentialStore {
    fn load(&self) -> Result<Option<String>, CredentialStoreError> {
        self.value
            .lock()
            .map(|value| value.clone())
            .map_err(|_| CredentialStoreError::Unavailable)
    }

    fn save(&self, value: &str) -> Result<(), CredentialStoreError> {
        self.value
            .lock()
            .map(|mut current| *current = Some(value.to_owned()))
            .map_err(|_| CredentialStoreError::Unavailable)
    }

    fn clear(&self) -> Result<(), CredentialStoreError> {
        self.value
            .lock()
            .map(|mut current| *current = None)
            .map_err(|_| CredentialStoreError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::{CredentialStore, MemoryCredentialStore};

    #[test]
    fn memory_store_round_trips_and_clears_a_credential() {
        let store = MemoryCredentialStore::default();
        assert_eq!(store.load().unwrap(), None);
        store.save("SESSDATA=redacted").unwrap();
        assert_eq!(store.load().unwrap().as_deref(), Some("SESSDATA=redacted"));
        store.clear().unwrap();
        assert_eq!(store.load().unwrap(), None);
    }
}
