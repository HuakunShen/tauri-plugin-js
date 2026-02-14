use tauri::{command, AppHandle, Runtime};

use std::collections::HashMap;

use crate::models::*;
use crate::JsExt;
use crate::Result;

#[command]
pub(crate) async fn spawn<R: Runtime>(
    app: AppHandle<R>,
    name: String,
    config: SpawnConfig,
) -> Result<ProcessInfo> {
    app.js().spawn(name, config).await
}

#[command]
pub(crate) async fn kill<R: Runtime>(app: AppHandle<R>, name: String) -> Result<()> {
    app.js().kill(name).await
}

#[command]
pub(crate) async fn kill_all<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    app.js().kill_all().await
}

#[command]
pub(crate) async fn restart<R: Runtime>(
    app: AppHandle<R>,
    name: String,
    config: Option<SpawnConfig>,
) -> Result<ProcessInfo> {
    app.js().restart(name, config).await
}

#[command]
pub(crate) async fn list_processes<R: Runtime>(app: AppHandle<R>) -> Result<Vec<ProcessInfo>> {
    app.js().list_processes().await
}

#[command]
pub(crate) async fn get_status<R: Runtime>(app: AppHandle<R>, name: String) -> Result<ProcessInfo> {
    app.js().get_status(name).await
}

#[command]
pub(crate) async fn write_stdin<R: Runtime>(
    app: AppHandle<R>,
    name: String,
    data: String,
) -> Result<()> {
    app.js().write_stdin(name, data).await
}

#[command]
pub(crate) async fn detect_runtimes<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Vec<RuntimeInfo>> {
    app.js().detect_runtimes().await
}

#[command]
pub(crate) async fn set_runtime_path<R: Runtime>(
    app: AppHandle<R>,
    runtime: String,
    path: String,
) -> Result<()> {
    app.js().set_runtime_path(runtime, path).await
}

#[command]
pub(crate) async fn get_runtime_paths<R: Runtime>(
    app: AppHandle<R>,
) -> Result<HashMap<String, String>> {
    app.js().get_runtime_paths().await
}
