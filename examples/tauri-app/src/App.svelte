<script lang="ts">
  import {
    spawn,
    kill,
    killAll,
    restart,
    listProcesses,
    onStdout,
    onStderr,
    onExit,
    createChannel,
    detectRuntimes,
    setRuntimePath,
    getRuntimePaths,
  } from "tauri-plugin-js-api";
  import type { RuntimeInfo, ProcessInfo } from "tauri-plugin-js-api";
  import type { BackendAPI } from "../backends/shared-api";
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { resolve, resolveResource } from "@tauri-apps/api/path";

  interface LogEntry {
    type: string;
    message: string;
    ts: string;
  }

  let logs = $state<LogEntry[]>([]);
  let processes = $state<ProcessInfo[]>([]);
  let channels = $state<Record<string, BackendAPI>>({});
  let runtimes = $state<RuntimeInfo[]>([]);
  let customPaths = $state<Record<string, string>>({});
  let showSettings = $state(false);
  let pollInterval: ReturnType<typeof setInterval> | null = null;

  function log(type: string, message: string) {
    const ts = new Date().toLocaleTimeString("en-US", {
      hour12: false,
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
      fractionalSecondDigits: 3,
    });
    logs = [...logs, { type, message, ts }];
    requestAnimationFrame(() => {
      const el = document.getElementById("log-panel");
      if (el) el.scrollTop = el.scrollHeight;
    });
  }

  function logColor(type: string): string {
    switch (type) {
      case "stdout":
        return "text-accent";
      case "stderr":
        return "text-danger";
      case "exit":
        return "text-warning";
      case "rpc":
        return "text-info";
      case "error":
        return "text-danger";
      case "system":
        return "text-text-muted";
      default:
        return "text-text";
    }
  }

  function logPrefix(type: string): string {
    switch (type) {
      case "stdout":
        return "OUT";
      case "stderr":
        return "ERR";
      case "exit":
        return "EXIT";
      case "rpc":
        return "RPC";
      case "error":
        return "ERR";
      case "system":
        return "SYS";
      default:
        return "---";
    }
  }

  function isRuntimeAvailable(name: string): boolean {
    return runtimes.find((r) => r.name === name)?.available ?? false;
  }

  function getRuntimeVersion(name: string): string | null {
    return runtimes.find((r) => r.name === name)?.version ?? null;
  }

  function getRuntimePath(name: string): string | null {
    return runtimes.find((r) => r.name === name)?.path ?? null;
  }

  async function loadRuntimes() {
    try {
      runtimes = await detectRuntimes();
      customPaths = await getRuntimePaths();
    } catch (e) {
      log("error", `failed to detect runtimes: ${e}`);
    }
  }

  async function refreshProcesses() {
    try {
      processes = await listProcesses();
    } catch {
      processes = [];
    }
  }

  // Map dev source filenames to bundled resource filenames
  const bundledNames: Record<string, string> = {
    "bun-worker.ts": "bun-worker.js",
    "node-worker.mjs": "node-worker.mjs",
    "deno-worker.ts": "deno-worker.ts",
  };

  async function resolveScript(filename: string): Promise<{ script: string; cwd?: string }> {
    if (import.meta.env.DEV) {
      return { script: filename, cwd: await resolve("..", "backends") };
    }
    const bundled = bundledNames[filename] ?? filename;
    const script = await resolveResource(`workers/${bundled}`);
    return { script };
  }

  async function spawnWorker(
    name: string,
    runtime: string,
    scriptPath: string,
  ) {
    try {
      const { script, cwd } = await resolveScript(scriptPath);
      const info = await spawn(name, {
        runtime: runtime as "bun" | "deno" | "node",
        script,
        cwd,
      });
      log("system", `spawned ${name} (pid: ${info.pid})`);

      onStdout(name, (data) => log("stdout", `[${name}] ${data}`));
      onStderr(name, (data) => log("stderr", `[${name}] ${data}`));
      onExit(name, (code) => {
        log("exit", `[${name}] exited with code ${code}`);
        delete channels[name];
        channels = { ...channels };
        refreshProcesses();
      });

      const { api } = await createChannel<Record<string, never>, BackendAPI>(
        name,
      );
      channels[name] = api;
      channels = { ...channels };
      log("system", `RPC channel ready for ${name}`);

      await refreshProcesses();
    } catch (e) {
      log("error", `failed to spawn ${name}: ${e}`);
    }
  }

  async function killProcess(name: string) {
    try {
      await kill(name);
      log("system", `killed ${name}`);
      delete channels[name];
      channels = { ...channels };
      await refreshProcesses();
    } catch (e) {
      log("error", `failed to kill ${name}: ${e}`);
    }
  }

  async function restartProcess(name: string) {
    try {
      delete channels[name];
      channels = { ...channels };
      const info = await restart(name);
      log("system", `restarted ${name} (pid: ${info.pid})`);

      onStdout(name, (data) => log("stdout", `[${name}] ${data}`));
      onStderr(name, (data) => log("stderr", `[${name}] ${data}`));
      onExit(name, (code) => {
        log("exit", `[${name}] exited with code ${code}`);
        delete channels[name];
        channels = { ...channels };
        refreshProcesses();
      });

      const { api } = await createChannel<Record<string, never>, BackendAPI>(
        name,
      );
      channels[name] = api;
      channels = { ...channels };
      log("system", `RPC channel ready for ${name}`);

      await refreshProcesses();
    } catch (e) {
      log("error", `failed to restart ${name}: ${e}`);
    }
  }

  async function killAllProcesses() {
    try {
      await killAll();
      log("system", "killed all processes");
      channels = {};
      await refreshProcesses();
    } catch (e) {
      log("error", `failed to kill all: ${e}`);
    }
  }

  async function typedRpcCall(
    name: string,
    label: string,
    fn: () => Promise<unknown>,
  ) {
    if (!channels[name]) {
      log("error", `no RPC channel for ${name}`);
      return;
    }
    try {
      const result = await fn();
      log("rpc", `[${name}] ${label} => ${JSON.stringify(result)}`);
    } catch (e) {
      log("error", `[${name}] ${label} failed: ${e}`);
    }
  }

  async function saveRuntimePath(runtime: string, path: string) {
    try {
      await setRuntimePath(runtime, path);
      customPaths = await getRuntimePaths();
      log(
        "system",
        path
          ? `set custom path for ${runtime}: ${path}`
          : `cleared custom path for ${runtime}`,
      );
    } catch (e) {
      log("error", `failed to set runtime path: ${e}`);
    }
  }

  let windowCounter = 0;
  async function openNewWindow() {
    windowCounter++;
    const label = `window-${windowCounter}`;
    new WebviewWindow(label, {
      title: `JS Runtime Manager - ${label}`,
      url: "/",
      width: 900,
      height: 700,
    });
    log("system", `opened new window: ${label}`);
  }

  function clearLogs() {
    logs = [];
  }

  const runtimeDotColor: Record<string, string> = {
    bun: "bg-warning",
    node: "bg-accent",
    deno: "bg-info",
  };

  const spawnConfigs = [
    { name: "bun-worker", runtime: "bun", script: "bun-worker.ts" },
    { name: "node-worker", runtime: "node", script: "node-worker.mjs" },
    { name: "deno-worker", runtime: "deno", script: "deno-worker.ts" },
  ];

  const binaryConfigs = [
    { name: "bun-compiled", sidecar: "bun-worker", label: "bun-worker (sidecar)" },
    { name: "deno-compiled", sidecar: "deno-worker", label: "deno-worker (sidecar)" },
  ];

  async function spawnBinary(name: string, sidecarName: string) {
    try {
      const info = await spawn(name, { sidecar: sidecarName });
      log("system", `spawned ${name} (pid: ${info.pid}) [sidecar]`);

      onStdout(name, (data) => log("stdout", `[${name}] ${data}`));
      onStderr(name, (data) => log("stderr", `[${name}] ${data}`));
      onExit(name, (code) => {
        log("exit", `[${name}] exited with code ${code}`);
        delete channels[name];
        channels = { ...channels };
        refreshProcesses();
      });

      const { api } = await createChannel<Record<string, never>, BackendAPI>(
        name,
      );
      channels[name] = api;
      channels = { ...channels };
      log("system", `RPC channel ready for ${name}`);
      await refreshProcesses();
    } catch (e) {
      log("error", `failed to spawn ${name}: ${e}`);
    }
  }

  $effect(() => {
    loadRuntimes();
    refreshProcesses();
    pollInterval = setInterval(refreshProcesses, 3000);
    return () => {
      if (pollInterval) clearInterval(pollInterval);
    };
  });
</script>

<div class="min-h-screen bg-background text-text font-mono">
  <!-- Header -->
  <header
    class="border-b border-border px-6 py-4 flex items-center justify-between"
  >
    <div class="flex items-center gap-3">
      <div class="w-2 h-2 rounded-full bg-accent animate-pulse"></div>
      <h1 class="text-lg font-semibold tracking-tight">
        <span class="text-accent">js</span><span class="text-text-muted"
          >::</span
        >runtime-manager
      </h1>
    </div>
    <div class="flex items-center gap-2">
      <button
        onclick={() => (showSettings = true)}
        class="px-3 py-1.5 text-xs border border-border-bright rounded-md hover:bg-surface-hover hover:border-accent/30 transition-colors cursor-pointer"
      >
        settings
      </button>
      <button
        onclick={openNewWindow}
        class="px-3 py-1.5 text-xs border border-border-bright rounded-md hover:bg-surface-hover hover:border-accent/30 transition-colors cursor-pointer"
      >
        + window
      </button>
      <button
        onclick={killAllProcesses}
        class="px-3 py-1.5 text-xs border border-danger-dim rounded-md text-danger hover:bg-danger-dim/30 transition-colors cursor-pointer"
      >
        kill all
      </button>
    </div>
  </header>

  <div class="flex h-[calc(100vh-65px)]">
    <!-- Left Panel: Controls -->
    <div class="w-80 border-r border-border flex flex-col overflow-y-auto">
      <!-- Spawn Section -->
      <div class="p-4 border-b border-border">
        <h2
          class="text-xs font-medium text-text-muted uppercase tracking-wider mb-3"
        >
          spawn
        </h2>
        <div class="space-y-2">
          {#each spawnConfigs as cfg}
            {@const available = isRuntimeAvailable(cfg.runtime)}
            {@const version = getRuntimeVersion(cfg.runtime)}
            <button
              onclick={() => spawnWorker(cfg.name, cfg.runtime, cfg.script)}
              disabled={!available}
              class="w-full px-3 py-2 text-sm text-left border border-border rounded-md transition-colors flex items-center gap-2 {available
                ? 'hover:bg-surface-hover hover:border-accent/30 cursor-pointer'
                : 'opacity-40 cursor-not-allowed'}"
            >
              <span
                class="w-1.5 h-1.5 rounded-full {available
                  ? (runtimeDotColor[cfg.runtime] ?? 'bg-text-dim')
                  : 'bg-danger'}"
              ></span>
              <span class="flex-1">{cfg.script}</span>
              {#if version}
                <span class="text-[10px] text-text-dim">{version}</span>
              {:else}
                <span class="text-[10px] text-danger">not found</span>
              {/if}
            </button>
          {/each}
        </div>
      </div>

      <!-- Sidecars -->
      <div class="p-4 border-b border-border border-dashed">
        <h2
          class="text-xs font-medium text-text-muted uppercase tracking-wider mb-3"
        >
          sidecars
        </h2>
        <div class="space-y-2">
          {#each binaryConfigs as cfg}
            <button
              onclick={() => spawnBinary(cfg.name, cfg.sidecar)}
              class="w-full px-3 py-2 text-sm text-left border border-border rounded-md transition-colors flex items-center gap-2 hover:bg-surface-hover hover:border-accent/30 cursor-pointer"
            >
              <span class="w-1.5 h-1.5 rounded-full bg-purple-400"></span>
              <span class="flex-1">{cfg.label}</span>
            </button>
          {/each}
        </div>
      </div>

      <!-- Process List -->
      <div class="p-4 border-b border-border">
        <h2
          class="text-xs font-medium text-text-muted uppercase tracking-wider mb-3"
        >
          processes <span class="text-text-dim">({processes.length})</span>
        </h2>
        {#if processes.length === 0}
          <p class="text-xs text-text-dim italic">no active processes</p>
        {:else}
          <div class="space-y-2">
            {#each processes as proc}
              <div class="border border-border rounded-md p-3">
                <div class="flex items-center justify-between mb-2">
                  <div class="flex items-center gap-2">
                    <span
                      class="w-1.5 h-1.5 rounded-full {proc.running
                        ? 'bg-accent'
                        : 'bg-danger'}"
                    ></span>
                    <span class="text-sm font-medium">{proc.name}</span>
                  </div>
                  <span class="text-[10px] text-text-dim font-mono"
                    >pid:{proc.pid || "?"}</span
                  >
                </div>
                <div class="flex gap-1">
                  <button
                    onclick={() => restartProcess(proc.name)}
                    class="flex-1 px-2 py-1 text-[10px] border border-border rounded hover:bg-surface-hover hover:border-warning/30 text-warning transition-colors cursor-pointer"
                  >
                    restart
                  </button>
                  <button
                    onclick={() => killProcess(proc.name)}
                    class="flex-1 px-2 py-1 text-[10px] border border-danger-dim rounded hover:bg-danger-dim/30 text-danger transition-colors cursor-pointer"
                  >
                    kill
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- RPC Calls -->
      <div class="p-4 flex-1">
        <h2
          class="text-xs font-medium text-text-muted uppercase tracking-wider mb-3"
        >
          rpc calls
        </h2>
        {#if Object.keys(channels).length === 0}
          <p class="text-xs text-text-dim italic">
            spawn a process to enable RPC
          </p>
        {:else}
          <div class="space-y-3">
            {#each Object.keys(channels) as name}
              <div class="space-y-1.5">
                <p class="text-[10px] text-text-muted uppercase tracking-wider">
                  {name}
                </p>
                <div class="grid grid-cols-2 gap-1">
                  <button
                    onclick={() =>
                      typedRpcCall(name, "add(5, 3)", () =>
                        channels[name].add(5, 3),
                      )}
                    class="px-2 py-1.5 text-[11px] border border-border rounded hover:bg-surface-hover hover:border-info/30 text-info transition-colors cursor-pointer"
                  >
                    add(5, 3)
                  </button>
                  <button
                    onclick={() =>
                      typedRpcCall(name, 'echo("hello")', () =>
                        channels[name].echo("hello"),
                      )}
                    class="px-2 py-1.5 text-[11px] border border-border rounded hover:bg-surface-hover hover:border-info/30 text-info transition-colors cursor-pointer"
                  >
                    echo("hello")
                  </button>
                  <button
                    onclick={() =>
                      typedRpcCall(name, "getSystemInfo()", () =>
                        channels[name].getSystemInfo(),
                      )}
                    class="px-2 py-1.5 text-[11px] border border-border rounded hover:bg-surface-hover hover:border-info/30 text-info transition-colors cursor-pointer"
                  >
                    getSystemInfo()
                  </button>
                  <button
                    onclick={() =>
                      typedRpcCall(name, "fibonacci(10)", () =>
                        channels[name].fibonacci(10),
                      )}
                    class="px-2 py-1.5 text-[11px] border border-border rounded hover:bg-surface-hover hover:border-info/30 text-info transition-colors cursor-pointer"
                  >
                    fibonacci(10)
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>

    <!-- Right Panel: Log -->
    <div class="flex-1 flex flex-col">
      <div
        class="px-4 py-2 border-b border-border flex items-center justify-between"
      >
        <h2
          class="text-xs font-medium text-text-muted uppercase tracking-wider"
        >
          output <span class="text-text-dim">({logs.length})</span>
        </h2>
        <button
          onclick={clearLogs}
          class="px-2 py-0.5 text-[10px] text-text-dim border border-border rounded hover:bg-surface-hover transition-colors cursor-pointer"
        >
          clear
        </button>
      </div>
      <div
        id="log-panel"
        class="flex-1 overflow-y-auto p-4 font-mono text-xs leading-relaxed"
      >
        {#if logs.length === 0}
          <p class="text-text-dim italic">awaiting output...</p>
        {:else}
          {#each logs as entry}
            <div
              class="flex gap-2 hover:bg-surface-hover/50 px-1 -mx-1 rounded"
            >
              <span class="text-text-dim shrink-0 select-none">{entry.ts}</span>
              <span
                class="shrink-0 w-8 text-right select-none {logColor(
                  entry.type,
                )}">{logPrefix(entry.type)}</span
              >
              <span class="{logColor(entry.type)} break-all"
                >{entry.message}</span
              >
            </div>
          {/each}
        {/if}
      </div>
    </div>
  </div>
</div>

<!-- Settings Dialog -->
{#if showSettings}
  <div class="fixed inset-0 z-50 flex items-center justify-center">
    <!-- Backdrop -->
    <button
      aria-label="Close settings"
      class="absolute inset-0 bg-black/60 cursor-default"
      onclick={() => (showSettings = false)}
    ></button>

    <!-- Modal -->
    <div
      class="relative bg-background border border-border rounded-lg shadow-2xl w-[520px] max-h-[80vh] overflow-y-auto"
    >
      <div
        class="px-6 py-4 border-b border-border flex items-center justify-between"
      >
        <h2 class="text-sm font-semibold">Runtime Settings</h2>
        <button
          onclick={() => (showSettings = false)}
          class="text-text-dim hover:text-text text-lg leading-none cursor-pointer"
          >&times;</button
        >
      </div>

      <div class="p-6 space-y-5">
        {#each ["bun", "node", "deno"] as rt}
          {@const info = runtimes.find((r) => r.name === rt)}
          {@const available = info?.available ?? false}
          <div class="border border-border rounded-md p-4 space-y-3">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2">
                <span
                  class="w-2 h-2 rounded-full {available
                    ? (runtimeDotColor[rt] ?? 'bg-accent')
                    : 'bg-danger'}"
                ></span>
                <span class="text-sm font-medium">{rt}</span>
              </div>
              {#if available}
                <span class="text-[10px] text-accent">available</span>
              {:else}
                <span class="text-[10px] text-danger">not found</span>
              {/if}
            </div>

            <div class="space-y-1 text-xs">
              <div class="flex gap-2">
                <span class="text-text-dim w-16 shrink-0">version:</span>
                <span class="text-text">{info?.version ?? "n/a"}</span>
              </div>
              <div class="flex gap-2">
                <span class="text-text-dim w-16 shrink-0">path:</span>
                <span class="text-text break-all">{info?.path ?? "n/a"}</span>
              </div>
            </div>

            <div class="flex gap-2">
              <input
                type="text"
                placeholder="custom executable path"
                value={customPaths[rt] ?? ""}
                oninput={(e: Event) => {
                  const target = e.target as HTMLInputElement;
                  customPaths = { ...customPaths, [rt]: target.value };
                }}
                class="flex-1 px-2 py-1.5 text-xs bg-surface border border-border rounded focus:border-accent/50 focus:outline-none placeholder:text-text-dim"
              />
              <button
                onclick={() => saveRuntimePath(rt, customPaths[rt] ?? "")}
                class="px-3 py-1.5 text-xs border border-border rounded hover:bg-surface-hover hover:border-accent/30 transition-colors cursor-pointer"
              >
                save
              </button>
            </div>
          </div>
        {/each}

        <button
          onclick={loadRuntimes}
          class="w-full px-3 py-2 text-xs border border-border-bright rounded-md hover:bg-surface-hover hover:border-accent/30 transition-colors cursor-pointer"
        >
          refresh detection
        </button>
      </div>
    </div>
  </div>
{/if}
