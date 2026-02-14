use serde::{ser::Serializer, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("process not found: {0}")]
    ProcessNotFound(String),
    #[error("process already exists: {0}")]
    ProcessAlreadyExists(String),
    #[error("process not running: {0}")]
    ProcessNotRunning(String),
    #[error("invalid config: {0}")]
    InvalidConfig(String),
    #[error("stdin write error for '{0}': {1}")]
    StdinWriteError(String, String),
    #[cfg(mobile)]
    #[error(transparent)]
    PluginInvoke(#[from] tauri::plugin::mobile::PluginInvokeError),
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
