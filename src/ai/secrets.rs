use std::collections::HashMap;
use std::sync::Mutex;

pub const OPENROUTER_API_KEY_NAME: &str = "openrouter_api_key";
pub const KEYRING_SERVICE: &str = "polit";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecureStorageError {
    Unavailable(String),
    Failed(String),
}

impl std::fmt::Display for SecureStorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecureStorageError::Unavailable(message) => {
                write!(f, "secure storage unavailable: {message}")
            }
            SecureStorageError::Failed(message) => write!(f, "secure storage failed: {message}"),
        }
    }
}

impl std::error::Error for SecureStorageError {}

pub trait SecureStorage: Send + Sync {
    fn read_secret(&self, name: &str) -> Result<Option<String>, SecureStorageError>;
    fn write_secret(&self, name: &str, secret: &str) -> Result<(), SecureStorageError>;
}

pub struct KeyringSecureStorage;

impl KeyringSecureStorage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for KeyringSecureStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl SecureStorage for KeyringSecureStorage {
    fn read_secret(&self, name: &str) -> Result<Option<String>, SecureStorageError> {
        ensure_native_keyring_store()?;
        let entry = keyring_core::Entry::new(KEYRING_SERVICE, name)
            .map_err(map_keyring_error_to_storage_error)?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(error) => Err(map_keyring_error_to_storage_error(error)),
        }
    }

    fn write_secret(&self, name: &str, secret: &str) -> Result<(), SecureStorageError> {
        ensure_native_keyring_store()?;
        let entry = keyring_core::Entry::new(KEYRING_SERVICE, name)
            .map_err(map_keyring_error_to_storage_error)?;
        entry
            .set_password(secret)
            .map_err(map_keyring_error_to_storage_error)
    }
}

pub fn save_openrouter_api_key(
    storage: &dyn SecureStorage,
    api_key: &str,
) -> Result<(), SecureStorageError> {
    storage.write_secret(OPENROUTER_API_KEY_NAME, api_key)
}

pub fn load_openrouter_api_key(
    storage: &dyn SecureStorage,
) -> Result<Option<String>, SecureStorageError> {
    storage.read_secret(OPENROUTER_API_KEY_NAME)
}

pub struct InMemorySecureStorage {
    secrets: Mutex<HashMap<String, String>>,
}

impl InMemorySecureStorage {
    pub fn new() -> Self {
        Self {
            secrets: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemorySecureStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl SecureStorage for InMemorySecureStorage {
    fn read_secret(&self, name: &str) -> Result<Option<String>, SecureStorageError> {
        let secrets = self
            .secrets
            .lock()
            .map_err(|error| SecureStorageError::Failed(error.to_string()))?;
        Ok(secrets.get(name).cloned())
    }

    fn write_secret(&self, name: &str, secret: &str) -> Result<(), SecureStorageError> {
        let mut secrets = self
            .secrets
            .lock()
            .map_err(|error| SecureStorageError::Failed(error.to_string()))?;
        secrets.insert(name.to_string(), secret.to_string());
        Ok(())
    }
}

fn ensure_native_keyring_store() -> Result<(), SecureStorageError> {
    #[cfg(target_os = "linux")]
    {
        return keyring::use_named_store("secret-service")
            .map_err(|error| SecureStorageError::Unavailable(error.to_string()));
    }

    #[cfg(any(
        target_os = "android",
        target_os = "freebsd",
        target_os = "macos",
        target_os = "openbsd",
        target_os = "windows",
    ))]
    {
        return keyring::use_native_store(false)
            .map_err(|error| SecureStorageError::Unavailable(error.to_string()));
    }

    #[cfg(not(any(
        target_os = "android",
        target_os = "freebsd",
        target_os = "linux",
        target_os = "macos",
        target_os = "openbsd",
        target_os = "windows",
    )))]
    {
        Err(SecureStorageError::Unavailable(
            "secure storage is not supported on this platform".to_string(),
        ))
    }
}

fn map_keyring_error_to_storage_error(error: keyring_core::Error) -> SecureStorageError {
    match error {
        keyring_core::Error::NoStorageAccess(error) => {
            SecureStorageError::Unavailable(error.to_string())
        }
        keyring_core::Error::NotSupportedByStore(message) => {
            SecureStorageError::Unavailable(message)
        }
        keyring_core::Error::NoDefaultStore => {
            SecureStorageError::Unavailable("secure storage is not initialized".to_string())
        }
        keyring_core::Error::PlatformFailure(error)
        | keyring_core::Error::BadDataFormat(_, error) => {
            SecureStorageError::Failed(error.to_string())
        }
        keyring_core::Error::BadEncoding(bytes) => {
            SecureStorageError::Failed(format!("invalid UTF-8 secret bytes: {bytes:?}"))
        }
        keyring_core::Error::TooLong(attr, limit) => {
            SecureStorageError::Failed(format!("attribute {attr} exceeds {limit} characters"))
        }
        keyring_core::Error::Invalid(attr, reason) => {
            SecureStorageError::Failed(format!("invalid attribute {attr}: {reason}"))
        }
        keyring_core::Error::Ambiguous(entries) => {
            SecureStorageError::Failed(format!("ambiguous secure storage entry: {entries:?}"))
        }
        keyring_core::Error::NoEntry => {
            SecureStorageError::Failed("secure storage entry is missing".to_string())
        }
        _ => SecureStorageError::Failed("unexpected secure storage error".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::config::{AiConfig, AiProviderKind};

    struct UnavailableStorage;

    impl SecureStorage for UnavailableStorage {
        fn read_secret(&self, _: &str) -> Result<Option<String>, SecureStorageError> {
            Err(SecureStorageError::Unavailable("keychain unavailable".to_string()))
        }

        fn write_secret(&self, _: &str, _: &str) -> Result<(), SecureStorageError> {
            Err(SecureStorageError::Unavailable("keychain unavailable".to_string()))
        }
    }

    #[test]
    fn openrouter_key_round_trips_through_abstract_storage_trait() {
        let storage = InMemorySecureStorage::new();

        save_openrouter_api_key(&storage, "sk-test-secret").unwrap();

        let loaded = load_openrouter_api_key(&storage).unwrap();
        assert_eq!(loaded.as_deref(), Some("sk-test-secret"));
    }

    #[test]
    fn config_payload_never_contains_openrouter_api_key() {
        let config = AiConfig {
            provider: AiProviderKind::OpenRouter,
            model: Some("openrouter/deepseek-r1".to_string()),
            openrouter_api_key: Some("sk-test-secret".to_string()),
        };

        let payload = toml::to_string(&config).unwrap();

        assert!(!payload.contains("sk-test-secret"));
        assert!(!payload.contains(OPENROUTER_API_KEY_NAME));
    }

    #[test]
    fn unavailable_secure_storage_returns_blocking_error() {
        let storage = UnavailableStorage;

        let error = save_openrouter_api_key(&storage, "sk-test-secret").unwrap_err();

        match error {
            SecureStorageError::Unavailable(message) => {
                assert!(message.contains("keychain unavailable"));
            }
            other => panic!("expected blocking unavailable error, got {other:?}"),
        }
    }
}
