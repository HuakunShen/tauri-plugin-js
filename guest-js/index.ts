import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// ── Types ──

export interface SpawnConfig {
  runtime?: "bun" | "deno" | "node";
  command?: string;
  script?: string;
  args?: string[];
  cwd?: string;
  env?: Record<string, string>;
}

export interface ProcessInfo {
  name: string;
  pid: number | null;
  running: boolean;
}

export interface StdioEventPayload {
  name: string;
  data: string;
}

export interface ExitEventPayload {
  name: string;
  code: number | null;
}

export interface RuntimeInfo {
  name: string;
  path: string | null;
  version: string | null;
  available: boolean;
}

// ── A) Command wrappers ──

export async function spawn(
  name: string,
  config: SpawnConfig
): Promise<ProcessInfo> {
  return invoke<ProcessInfo>("plugin:js|spawn", { name, config });
}

export async function kill(name: string): Promise<void> {
  return invoke<void>("plugin:js|kill", { name });
}

export async function killAll(): Promise<void> {
  return invoke<void>("plugin:js|kill_all");
}

export async function restart(
  name: string,
  config?: SpawnConfig
): Promise<ProcessInfo> {
  return invoke<ProcessInfo>("plugin:js|restart", { name, config: config ?? null });
}

export async function listProcesses(): Promise<ProcessInfo[]> {
  return invoke<ProcessInfo[]>("plugin:js|list_processes");
}

export async function getStatus(name: string): Promise<ProcessInfo> {
  return invoke<ProcessInfo>("plugin:js|get_status", { name });
}

export async function writeStdin(name: string, data: string): Promise<void> {
  return invoke<void>("plugin:js|write_stdin", { name, data });
}

export async function detectRuntimes(): Promise<RuntimeInfo[]> {
  return invoke<RuntimeInfo[]>("plugin:js|detect_runtimes");
}

export async function setRuntimePath(
  runtime: string,
  path: string
): Promise<void> {
  return invoke<void>("plugin:js|set_runtime_path", { runtime, path });
}

export async function getRuntimePaths(): Promise<Record<string, string>> {
  return invoke<Record<string, string>>("plugin:js|get_runtime_paths");
}

// ── B) Event helpers ──

export function onStdout(
  name: string,
  callback: (data: string) => void
): Promise<UnlistenFn> {
  return listen<StdioEventPayload>("js-process-stdout", (event) => {
    if (event.payload.name === name) {
      callback(event.payload.data);
    }
  });
}

export function onStderr(
  name: string,
  callback: (data: string) => void
): Promise<UnlistenFn> {
  return listen<StdioEventPayload>("js-process-stderr", (event) => {
    if (event.payload.name === name) {
      callback(event.payload.data);
    }
  });
}

export function onExit(
  name: string,
  callback: (code: number | null) => void
): Promise<UnlistenFn> {
  return listen<ExitEventPayload>("js-process-exit", (event) => {
    if (event.payload.name === name) {
      callback(event.payload.code);
    }
  });
}

// ── C) JsRuntimeIo class (kkrpc IoInterface via structural typing) ──

type MessageListener = (data: string) => void;

export class JsRuntimeIo {
  readonly name: string;
  private processName: string;
  private queue: string[] = [];
  private waitResolve: ((value: string | null) => void) | null = null;
  private listeners: Set<MessageListener> = new Set();
  private unlisten: UnlistenFn | null = null;
  private _isDestroyed = false;

  constructor(processName: string) {
    this.processName = processName;
    this.name = `tauri-js-runtime:${processName}`;
  }

  get isDestroyed(): boolean {
    return this._isDestroyed;
  }

  async initialize(): Promise<void> {
    this.unlisten = await listen<StdioEventPayload>(
      "js-process-stdout",
      (event) => {
        if (event.payload.name !== this.processName) return;
        if (this._isDestroyed) return;

        // Re-append the newline that BufReader::lines() strips
        const data = event.payload.data + "\n";

        // Dispatch to message listeners
        for (const listener of this.listeners) {
          listener(data);
        }

        // Feed the read queue
        if (this.waitResolve) {
          const resolve = this.waitResolve;
          this.waitResolve = null;
          resolve(data);
        } else {
          this.queue.push(data);
        }
      }
    );
  }

  async write(data: string): Promise<void> {
    await writeStdin(this.processName, data);
  }

  async read(): Promise<string | null> {
    if (this._isDestroyed) {
      // Return a never-resolving promise so kkrpc's listen loop hangs
      return new Promise<string | null>(() => {});
    }

    if (this.queue.length > 0) {
      return this.queue.shift()!;
    }

    return new Promise<string | null>((resolve) => {
      this.waitResolve = resolve;
    });
  }

  on(event: "message" | "error", listener: MessageListener): void {
    if (event === "message") {
      this.listeners.add(listener);
    }
  }

  off(event: "message" | "error", listener: Function): void {
    if (event === "message") {
      this.listeners.delete(listener as MessageListener);
    }
  }

  async destroy(): Promise<void> {
    this._isDestroyed = true;
    if (this.unlisten) {
      this.unlisten();
      this.unlisten = null;
    }
    if (this.waitResolve) {
      this.waitResolve(null);
      this.waitResolve = null;
    }
    this.listeners.clear();
    this.queue = [];
  }
}

// ── D) Channel helper (dynamic kkrpc import) ──

export async function createChannel<
  LocalAPI extends Record<string, any> = Record<string, never>,
  RemoteAPI extends Record<string, any> = Record<string, any>
>(
  processName: string,
  localApi?: LocalAPI
): Promise<{
  channel: any;
  api: RemoteAPI;
  io: JsRuntimeIo;
}> {
  const { RPCChannel } = await import("kkrpc/browser");
  const io = new JsRuntimeIo(processName);
  await io.initialize();
  const channel = new RPCChannel<LocalAPI, RemoteAPI>(io as any, { expose: localApi ?? ({} as LocalAPI) });
  const api = channel.getAPI();
  return { channel, api: api as RemoteAPI, io };
}
