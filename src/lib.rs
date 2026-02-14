use tauri::{
  plugin::{Builder, TauriPlugin},
  Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::Js;
#[cfg(mobile)]
use mobile::Js;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the js APIs.
pub trait JsExt<R: Runtime> {
  fn js(&self) -> &Js<R>;
}

impl<R: Runtime, T: Manager<R>> crate::JsExt<R> for T {
  fn js(&self) -> &Js<R> {
    self.state::<Js<R>>().inner()
  }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
  Builder::new("js")
    .invoke_handler(tauri::generate_handler![commands::ping])
    .setup(|app, api| {
      #[cfg(mobile)]
      let js = mobile::init(app, api)?;
      #[cfg(desktop)]
      let js = desktop::init(app, api)?;
      app.manage(js);
      Ok(())
    })
    .build()
}
