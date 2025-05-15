use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, NetworkError>;

impl From<zbus::Error> for NetworkError {
    fn from(err: zbus::Error) -> Self {
        match err {
            zbus::Error::Failure(msg) => NetworkError::OperationError(msg.to_string()),
            _ => NetworkError::ZBusError(err.to_string()),
        }
    }
}

impl From<zbus::fdo::Error> for NetworkError {
    fn from(err: zbus::fdo::Error) -> Self {
        NetworkError::OperationError(err.to_string())
    }
}

// Display implementation is now derived by thiserror

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum NetworkError {
    #[error("Unsupported WiFi security type")]
    UnsupportedSecurityType,
    #[error("ZBus error: {0}")]
    ZBusError(String),
    #[error(transparent)]
    #[serde(skip_serializing, skip_deserializing)]
    Io(#[from] std::io::Error),

    #[error("Failed to initialize NetworkManager")]
    InitializationError,
    
    #[error("Network operation failed")]
    OperationError(String),
    
    #[error("No network connection available")]
    NoConnection,
    
    #[error("Network connection failed")]
    ConnectionFailed(String),
    
    #[error("NetworkManager not initialized")]
    NotInitialized,
    
    #[error("Failed to acquire lock on network manager")]
    LockError,
    
    #[error("Feature not implemented")]
    NotImplemented,
    
    #[error("Task execution failed")]
    TaskError,
    
    #[error("Permission denied")]
    PermissionDenied,
    
    #[error("Failed to create runtime")]
    RuntimeError,
}
