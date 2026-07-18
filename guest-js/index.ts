import type { RPCChannel, RPCMessage, Transport } from "kkrpc/browser";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// ── Types ──

export interface SpawnConfig {
  runtime?: "bun" | "deno" | "node";
  command?: string;
  sidecar?: string;
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
  config: SpawnConfig,
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
  config?: SpawnConfig,
): Promise<ProcessInfo> {
  return invoke<ProcessInfo>("plugin:js|restart", {
    name,
    config: config ?? null,
  });
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
  path: string,
): Promise<void> {
  return invoke<void>("plugin:js|set_runtime_path", { runtime, path });
}

export async function getRuntimePaths(): Promise<Record<string, string>> {
  return invoke<Record<string, string>>("plugin:js|get_runtime_paths");
}

// ── B) Event helpers ──

export function onStdout(
  name: string,
  callback: (data: string) => void,
): Promise<UnlistenFn> {
  return listen<StdioEventPayload>("js-process-stdout", (event) => {
    if (event.payload.name === name) {
      callback(event.payload.data);
    }
  });
}

export function onStderr(
  name: string,
  callback: (data: string) => void,
): Promise<UnlistenFn> {
  return listen<StdioEventPayload>("js-process-stderr", (event) => {
    if (event.payload.name === name) {
      callback(event.payload.data);
    }
  });
}

export function onExit(
  name: string,
  callback: (code: number | null) => void,
): Promise<UnlistenFn> {
  return listen<ExitEventPayload>("js-process-exit", (event) => {
    if (event.payload.name === name) {
      callback(event.payload.code);
    }
  });
}

// ── C) Native kkrpc transport over the process's stdio ──

export async function jsRuntimeTransport(
  processName: string,
): Promise<Transport<RPCMessage>> {
  const [{ createTransport }, { jsonLineCodec }] = await Promise.all([
    import("kkrpc/transport"),
    import("kkrpc/codecs"),
  ]);

  const messageListeners = new Set<(wire: string) => void>();
  const closeListeners = new Set<(reason?: Error) => void>();
  let exited = false;
  let exitReason: Error | undefined;

  // Rust's BufReader::lines() strips the trailing \n; jsonLineCodec's decode
  // tolerates that, so each stdout event forwards as one wire frame.
  const unlistenStdout = await listen<StdioEventPayload>(
    "js-process-stdout",
    (event) => {
      if (event.payload.name !== processName) return;
      for (const listener of messageListeners) {
        listener(event.payload.data);
      }
    },
  );

  const unlistenExit = await listen<ExitEventPayload>(
    "js-process-exit",
    (event) => {
      if (event.payload.name !== processName || exited) return;
      exited = true;
      exitReason =
        event.payload.code === 0
          ? undefined
          : new Error(
              `process "${processName}" exited with code ${event.payload.code}`,
            );
      for (const listener of closeListeners) {
        listener(exitReason);
      }
      closeListeners.clear();
    },
  );

  return createTransport<RPCMessage, string>({
    platform: {
      send: (wire) => writeStdin(processName, wire),
      subscribe(listener) {
        messageListeners.add(listener);
        return () => {
          messageListeners.delete(listener);
        };
      },
      close() {
        unlistenStdout();
        unlistenExit();
        messageListeners.clear();
        closeListeners.clear();
      },
      onClose(listener) {
        if (exited) {
          queueMicrotask(() => listener(exitReason));
          return () => {};
        }
        closeListeners.add(listener);
        return () => {
          closeListeners.delete(listener);
        };
      },
    },
    codec: jsonLineCodec<RPCMessage>(),
  });
}

// ── D) Channel helper (dynamic kkrpc import) ──

export async function createChannel<
  LocalAPI extends Record<string, any> = Record<string, never>,
  RemoteAPI extends Record<string, any> = Record<string, any>,
>(
  processName: string,
  localApi?: LocalAPI,
): Promise<{
  channel: RPCChannel<LocalAPI, RemoteAPI>;
  api: RemoteAPI;
  transport: Transport<RPCMessage>;
}> {
  const { RPCChannel } = await import("kkrpc/browser");
  const transport = await jsRuntimeTransport(processName);
  const channel = new RPCChannel<LocalAPI, RemoteAPI>(transport, {
    expose: localApi ?? ({} as LocalAPI),
  });
  const api = channel.getAPI();
  return { channel, api: api as RemoteAPI, transport };
}
