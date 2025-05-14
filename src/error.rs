use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, NetworkError>;

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum NetworkError {
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
    
    #[error("Failed to acquire lock on network manager")]
    LockError,
    
    #[error("Task execution failed")]
    TaskError,
    
    #[error("Permission denied")]
    PermissionDenied,
}
