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

fn build_spawn_program(config: &SpawnConfig) -> crate::Result<(String, Vec<String>)> {
    if let Some(ref command) = config.command {
        return Ok((command.clone(), config.args.clone().unwrap_or_default()));
    }

    let runtime = config.runtime.as_ref().ok_or_else(|| {
        crate::Error::InvalidConfig(
            "either 'sidecar', 'command', or 'runtime' must be specified".to_string(),
        )
    })?;

    let mut args = match runtime.as_str() {
        "bun" | "node" => Vec::new(),
        "deno" => vec!["run".to_string(), "-A".to_string()],
        other => {
            return Err(crate::Error::InvalidConfig(format!(
                "unknown runtime: {}",
                other
            )));
        }
    };

    if let Some(ref script) = config.script {
        args.push(script.clone());
    }
    if let Some(ref extra) = config.args {
        args.extend(extra.iter().cloned());
    }

    Ok((runtime.clone(), args))
}

fn build_sidecar_spawn_program(program: String, config: &SpawnConfig) -> (String, Vec<String>) {
    (program, config.args.clone().unwrap_or_default())
}

fn sidecar_candidate_paths(
    exe_dir: &std::path::Path,
    name: &str,
    target_triple: &str,
) -> Vec<std::path::PathBuf> {
    let mut candidates = vec![exe_dir.join(name)];

    #[cfg(windows)]
    candidates.push(exe_dir.join(format!("{name}.exe")));

    candidates.push(exe_dir.join(format!("{name}-{target_triple}")));

    #[cfg(windows)]
    candidates.push(exe_dir.join(format!("{name}-{target_triple}.exe")));

    candidates
}

fn resolve_program_with_override(
    runtime: Option<&str>,
    program: String,
    overrides: &std::collections::HashMap<String, String>,
) -> String {
    if let Some(name) = runtime {
        return overrides.get(name).cloned().unwrap_or(program);
    }

    program
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
        let (program, args_vec) = if let Some(ref sidecar) = config.sidecar {
            let path = self.resolve_sidecar(sidecar)?;
            build_sidecar_spawn_program(path.to_string_lossy().to_string(), &config)
        } else {
            build_spawn_program(&config)?
        };

        // Apply custom runtime path override if configured
        let program = {
            let custom_paths = self.runtime_paths.lock().await;
            resolve_program_with_override(config.runtime.as_deref(), program, &custom_paths)
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

    fn resolve_sidecar(&self, name: &str) -> crate::Result<std::path::PathBuf> {
        let current_exe = std::env::current_exe().map_err(crate::Error::Io)?;
        let exe_dir = current_exe.parent().ok_or_else(|| {
            crate::Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "could not determine executable directory",
            ))
        })?;

        if let Some(candidate) =
            sidecar_candidate_paths(exe_dir, name, env!("TARGET_TRIPLE"))
                .into_iter()
                .find(|candidate| candidate.exists())
        {
            return Ok(candidate);
        }

        Err(crate::Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("sidecar not found: {name} (looked in {})", exe_dir.display()),
        )))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn_config() -> SpawnConfig {
        SpawnConfig {
            runtime: None,
            command: None,
            sidecar: None,
            script: None,
            args: None,
            cwd: None,
            env: None,
        }
    }

    #[test]
    fn builds_bun_command_with_script_and_extra_args() {
        let mut config = spawn_config();
        config.runtime = Some("bun".to_string());
        config.script = Some("worker.ts".to_string());
        config.args = Some(vec!["--watch".to_string()]);

        let (program, args) = build_spawn_program(&config).unwrap();

        assert_eq!(program, "bun");
        assert_eq!(args, vec!["worker.ts", "--watch"]);
    }

    #[test]
    fn builds_bun_command_without_script() {
        let mut config = spawn_config();
        config.runtime = Some("bun".to_string());

        let (program, args) = build_spawn_program(&config).unwrap();

        assert_eq!(program, "bun");
        assert!(args.is_empty());
    }

    #[test]
    fn builds_deno_command_with_script() {
        let mut config = spawn_config();
        config.runtime = Some("deno".to_string());
        config.script = Some("main.ts".to_string());

        let (program, args) = build_spawn_program(&config).unwrap();

        assert_eq!(program, "deno");
        assert_eq!(args, vec!["run", "-A", "main.ts"]);
    }

    #[test]
    fn builds_node_command_with_script_and_extra_args() {
        let mut config = spawn_config();
        config.runtime = Some("node".to_string());
        config.script = Some("server.mjs".to_string());
        config.args = Some(vec!["--port".to_string(), "8080".to_string()]);

        let (program, args) = build_spawn_program(&config).unwrap();

        assert_eq!(program, "node");
        assert_eq!(args, vec!["server.mjs", "--port", "8080"]);
    }

    #[test]
    fn builds_direct_command_path() {
        let mut config = spawn_config();
        config.command = Some("/usr/local/bin/custom".to_string());
        config.args = Some(vec!["--flag".to_string()]);

        let (program, args) = build_spawn_program(&config).unwrap();

        assert_eq!(program, "/usr/local/bin/custom");
        assert_eq!(args, vec!["--flag"]);
    }

    #[test]
    fn builds_sidecar_command_with_extra_args() {
        let mut config = spawn_config();
        config.sidecar = Some("worker".to_string());
        config.args = Some(vec!["--flag".to_string()]);

        let (program, args) = build_sidecar_spawn_program("/app/worker".to_string(), &config);

        assert_eq!(program, "/app/worker");
        assert_eq!(args, vec!["--flag"]);
    }

    #[test]
    fn sidecar_candidates_include_plain_and_triple_names() {
        let dir = std::path::PathBuf::from("/opt/app");
        let candidates = sidecar_candidate_paths(&dir, "my-worker", "aarch64-apple-darwin");
        let rendered: Vec<String> = candidates
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        assert!(rendered.contains(&"/opt/app/my-worker".to_string()));
        assert!(rendered.contains(&"/opt/app/my-worker-aarch64-apple-darwin".to_string()));
    }

    #[test]
    fn sidecar_candidates_canonical_order_is_plain_first() {
        let dir = std::path::PathBuf::from("/opt/app");
        let candidates = sidecar_candidate_paths(&dir, "w", "x86_64-unknown-linux-gnu");
        assert_eq!(candidates[0].to_string_lossy().to_string(), "/opt/app/w");
    }

    #[test]
    fn runtime_override_replaces_program() {
        let program = "bun".to_string();
        let overrides = std::collections::HashMap::from([(
            "bun".to_string(),
            "/opt/homebrew/bin/bun".to_string(),
        )]);

        let resolved = resolve_program_with_override(Some("bun"), program, &overrides);

        assert_eq!(resolved, "/opt/homebrew/bin/bun");
    }

    #[test]
    fn runtime_override_passthrough_when_missing() {
        let program = "deno".to_string();
        let overrides = std::collections::HashMap::new();

        let resolved = resolve_program_with_override(Some("deno"), program.clone(), &overrides);

        assert_eq!(resolved, program);
    }

    #[test]
    fn rejects_unknown_runtime() {
        let mut config = spawn_config();
        config.runtime = Some("qjs".to_string());

        let error = build_spawn_program(&config).unwrap_err();

        match error {
            crate::Error::InvalidConfig(message) => assert!(message.contains("qjs")),
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }

    #[test]
    fn rejects_missing_spawn_target() {
        let error = build_spawn_program(&spawn_config()).unwrap_err();

        match error {
            crate::Error::InvalidConfig(message) => {
                assert_eq!(
                    message,
                    "either 'sidecar', 'command', or 'runtime' must be specified"
                );
            }
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }
}
