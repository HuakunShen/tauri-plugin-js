use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, RunEvent, Runtime,
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
        .invoke_handler(tauri::generate_handler![
            commands::spawn,
            commands::kill,
            commands::kill_all,
            commands::restart,
            commands::list_processes,
            commands::get_status,
            commands::write_stdin,
            commands::detect_runtimes,
            commands::set_runtime_path,
            commands::get_runtime_paths,
        ])
        .setup(|app, api| {
            #[cfg(mobile)]
            let js = mobile::init(app, api)?;
            #[cfg(desktop)]
            let js = desktop::init(app, api)?;
            app.manage(js);
            Ok(())
        })
        .on_event(|app, event| {
            if let RunEvent::Exit = event {
                let js = app.state::<Js<R>>();
                tauri::async_runtime::block_on(async {
                    let _ = js.kill_all().await;
                });
            }
        })
        .build()
}
