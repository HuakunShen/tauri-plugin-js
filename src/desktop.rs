use std::collections::HashMap;
use std::sync::Arc;

use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Emitter, Runtime};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::Mutex;

use crate::models::*;

struct ProcessEntry {
    child: Child,
    stdin: Option<ChildStdin>,
    config: SpawnConfig,
}

pub struct Js<R: Runtime> {
    app: AppHandle<R>,
    processes: Arc<Mutex<HashMap<String, ProcessEntry>>>,
    runtime_paths: Arc<Mutex<HashMap<String, String>>>,
}

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<Js<R>> {
    Ok(Js {
        app: app.clone(),
        processes: Arc::new(Mutex::new(HashMap::new())),
        runtime_paths: Arc::new(Mutex::new(HashMap::new())),
    })
}

impl<R: Runtime> Js<R> {
    pub async fn spawn(&self, name: String, config: SpawnConfig) -> crate::Result<ProcessInfo> {
        // Check if process already exists
        {
            let procs = self.processes.lock().await;
            if procs.contains_key(&name) {
                return Err(crate::Error::ProcessAlreadyExists(name));
            }
        }

        // Build the command
        let (program, mut args_vec) = if let Some(ref cmd) = config.command {
            (cmd.clone(), Vec::new())
        } else if let Some(ref runtime) = config.runtime {
            match runtime.as_str() {
                "bun" => {
                    let mut a = Vec::new();
                    if let Some(ref script) = config.script {
                        a.push(script.clone());
                    }
                    ("bun".to_string(), a)
                }
                "deno" => {
                    let mut a = vec!["run".to_string(), "-A".to_string()];
                    if let Some(ref script) = config.script {
                        a.push(script.clone());
                    }
                    ("deno".to_string(), a)
                }
                "node" => {
                    let mut a = Vec::new();
                    if let Some(ref script) = config.script {
                        a.push(script.clone());
                    }
                    ("node".to_string(), a)
                }
                other => {
                    return Err(crate::Error::InvalidConfig(format!(
                        "unknown runtime: {}",
                        other
                    )));
                }
            }
        } else {
            return Err(crate::Error::InvalidConfig(
                "either 'runtime' or 'command' must be specified".to_string(),
            ));
        };

        // Append extra args
        if let Some(ref extra) = config.args {
            args_vec.extend(extra.iter().cloned());
        }

        // Apply custom runtime path override if configured
        let program = {
            let custom_paths = self.runtime_paths.lock().await;
            if let Some(ref runtime) = config.runtime {
                custom_paths.get(runtime).cloned().unwrap_or(program)
            } else {
                program
            }
        };

        let mut cmd = Command::new(&program);
        cmd.args(&args_vec);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        if let Some(ref cwd) = config.cwd {
            cmd.current_dir(cwd);
        }
        if let Some(ref env) = config.env {
            for (k, v) in env {
                cmd.env(k, v);
            }
        }

        let mut child = cmd.spawn().map_err(crate::Error::Io)?;

        let pid = child.id();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let stdin = child.stdin.take();

        let entry = ProcessEntry {
            child,
            stdin,
            config: config.clone(),
        };

        {
            let mut procs = self.processes.lock().await;
            procs.insert(name.clone(), entry);
        }

        // Spawn stdout reader task
        if let Some(stdout) = stdout {
            let app = self.app.clone();
            let proc_name = name.clone();
            tauri::async_runtime::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let payload = StdioEventPayload {
                        name: proc_name.clone(),
                        data: line,
                    };
                    let _ = app.emit("js-process-stdout", &payload);
                }
            });
        }

        // Spawn stderr reader task
        if let Some(stderr) = stderr {
            let app = self.app.clone();
            let proc_name = name.clone();
            tauri::async_runtime::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let payload = StdioEventPayload {
                        name: proc_name.clone(),
                        data: line,
                    };
                    let _ = app.emit("js-process-stderr", &payload);
                }
            });
        }

        // Spawn exit watcher task
        {
            let app = self.app.clone();
            let proc_name = name.clone();
            let processes = self.processes.clone();
            tauri::async_runtime::spawn(async move {
                // Wait for the child to exit by polling its status
                loop {
                    let exit_status = {
                        let mut procs = processes.lock().await;
                        if let Some(entry) = procs.get_mut(&proc_name) {
                            match entry.child.try_wait() {
                                Ok(Some(status)) => Some(status.code()),
                                Ok(None) => None,
                                Err(_) => {
                                    // Process errored, treat as exited
                                    Some(None)
                                }
                            }
                        } else {
                            // Entry was removed (killed), stop watching
                            break;
                        }
                    };

                    if let Some(code) = exit_status {
                        // Remove from map
                        {
                            let mut procs = processes.lock().await;
                            procs.remove(&proc_name);
                        }
                        let payload = ExitEventPayload {
                            name: proc_name,
                            code,
                        };
                        let _ = app.emit("js-process-exit", &payload);
                        break;
                    }

                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            });
        }

        Ok(ProcessInfo {
            name,
            pid,
            running: true,
        })
    }

    pub async fn kill(&self, name: String) -> crate::Result<()> {
        let mut entry = {
            let mut procs = self.processes.lock().await;
            procs
                .remove(&name)
                .ok_or_else(|| crate::Error::ProcessNotFound(name.clone()))?
        };

        // Drop stdin first to signal EOF
        entry.stdin.take();
        // Kill the child outside the lock
        let _ = entry.child.kill().await;
        Ok(())
    }

    pub async fn kill_all(&self) -> crate::Result<()> {
        let entries: Vec<(String, ProcessEntry)> = {
            let mut procs = self.processes.lock().await;
            procs.drain().collect()
        };

        for (_, mut entry) in entries {
            entry.stdin.take();
            let _ = entry.child.kill().await;
        }
        Ok(())
    }

    pub async fn restart(
        &self,
        name: String,
        config: Option<SpawnConfig>,
    ) -> crate::Result<ProcessInfo> {
        // Get the old config before killing
        let old_config = {
            let procs = self.processes.lock().await;
            procs
                .get(&name)
                .map(|e| e.config.clone())
                .ok_or_else(|| crate::Error::ProcessNotFound(name.clone()))?
        };

        self.kill(name.clone()).await?;
        let spawn_config = config.unwrap_or(old_config);
        self.spawn(name, spawn_config).await
    }

    pub async fn list_processes(&self) -> crate::Result<Vec<ProcessInfo>> {
        let procs = self.processes.lock().await;
        let mut list = Vec::new();
        for (name, entry) in procs.iter() {
            list.push(ProcessInfo {
                name: name.clone(),
                pid: entry.child.id(),
                running: true,
            });
        }
        Ok(list)
    }

    pub async fn get_status(&self, name: String) -> crate::Result<ProcessInfo> {
        let procs = self.processes.lock().await;
        let entry = procs
            .get(&name)
            .ok_or_else(|| crate::Error::ProcessNotFound(name.clone()))?;
        Ok(ProcessInfo {
            name,
            pid: entry.child.id(),
            running: true,
        })
    }

    pub async fn write_stdin(&self, name: String, data: String) -> crate::Result<()> {
        let mut procs = self.processes.lock().await;
        let entry = procs
            .get_mut(&name)
            .ok_or_else(|| crate::Error::ProcessNotFound(name.clone()))?;
        let stdin = entry
            .stdin
            .as_mut()
            .ok_or_else(|| crate::Error::ProcessNotRunning(name.clone()))?;
        stdin
            .write_all(data.as_bytes())
            .await
            .map_err(|e| crate::Error::StdinWriteError(name.clone(), e.to_string()))?;
        stdin
            .flush()
            .await
            .map_err(|e| crate::Error::StdinWriteError(name, e.to_string()))?;
        Ok(())
    }

    pub async fn detect_runtimes(&self) -> crate::Result<Vec<RuntimeInfo>> {
        let runtimes = ["bun", "node", "deno"];
        let mut results = Vec::new();

        for rt in &runtimes {
            let version = tokio::process::Command::new(rt)
                .arg("--version")
                .output()
                .await
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                    } else {
                        None
                    }
                });

            let path = tokio::process::Command::new("which")
                .arg(rt)
                .output()
                .await
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                    } else {
                        None
                    }
                });

            let available = version.is_some();
            results.push(RuntimeInfo {
                name: rt.to_string(),
                path,
                version,
                available,
            });
        }

        Ok(results)
    }

    pub async fn set_runtime_path(
        &self,
        runtime: String,
        path: String,
    ) -> crate::Result<()> {
        let mut paths = self.runtime_paths.lock().await;
        if path.is_empty() {
            paths.remove(&runtime);
        } else {
            paths.insert(runtime, path);
        }
        Ok(())
    }

    pub async fn get_runtime_paths(&self) -> crate::Result<HashMap<String, String>> {
        let paths = self.runtime_paths.lock().await;
        Ok(paths.clone())
    }
}
