use serde::de::DeserializeOwned;
use tauri::{
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime,
};

use crate::models::*;

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_js);

pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> crate::Result<Js<R>> {
    #[cfg(target_os = "android")]
    let handle = api.register_android_plugin("", "ExamplePlugin")?;
    #[cfg(target_os = "ios")]
    let handle = api.register_ios_plugin(init_plugin_js)?;
    Ok(Js(handle))
}

/// Access to the js APIs.
pub struct Js<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> Js<R> {
    pub async fn spawn(&self, _name: String, _config: SpawnConfig) -> crate::Result<ProcessInfo> {
        Err(crate::Error::InvalidConfig(
            "JS process management is not supported on mobile".to_string(),
        ))
    }

    pub async fn kill(&self, _name: String) -> crate::Result<()> {
        Err(crate::Error::InvalidConfig(
            "JS process management is not supported on mobile".to_string(),
        ))
    }

    pub async fn kill_all(&self) -> crate::Result<()> {
        Err(crate::Error::InvalidConfig(
            "JS process management is not supported on mobile".to_string(),
        ))
    }

    pub async fn restart(
        &self,
        _name: String,
        _config: Option<SpawnConfig>,
    ) -> crate::Result<ProcessInfo> {
        Err(crate::Error::InvalidConfig(
            "JS process management is not supported on mobile".to_string(),
        ))
    }

    pub async fn list_processes(&self) -> crate::Result<Vec<ProcessInfo>> {
        Err(crate::Error::InvalidConfig(
            "JS process management is not supported on mobile".to_string(),
        ))
    }

    pub async fn get_status(&self, _name: String) -> crate::Result<ProcessInfo> {
        Err(crate::Error::InvalidConfig(
            "JS process management is not supported on mobile".to_string(),
        ))
    }

    pub async fn write_stdin(&self, _name: String, _data: String) -> crate::Result<()> {
        Err(crate::Error::InvalidConfig(
            "JS process management is not supported on mobile".to_string(),
        ))
    }

    pub async fn detect_runtimes(&self) -> crate::Result<Vec<RuntimeInfo>> {
        Err(crate::Error::InvalidConfig(
            "JS process management is not supported on mobile".to_string(),
        ))
    }

    pub async fn set_runtime_path(
        &self,
        _runtime: String,
        _path: String,
    ) -> crate::Result<()> {
        Err(crate::Error::InvalidConfig(
            "JS process management is not supported on mobile".to_string(),
        ))
    }

    pub async fn get_runtime_paths(
        &self,
    ) -> crate::Result<std::collections::HashMap<String, String>> {
        Err(crate::Error::InvalidConfig(
            "JS process management is not supported on mobile".to_string(),
        ))
    }
}
