import { RPCChannel, BunIo } from "kkrpc";
import type { BackendAPI } from "./shared-api";

function fibonacci(n: number): number {
  if (n <= 1) return n;
  return fibonacci(n - 1) + fibonacci(n - 2);
}

const api: BackendAPI = {
  async add(a: number, b: number) {
    return a + b;
  },
  async echo(message: string) {
    return `[bun] ${message}`;
  },
  async getSystemInfo() {
    return {
      runtime: "bun",
      pid: process.pid,
      platform: process.platform,
      arch: process.arch,
    };
  },
  async fibonacci(n: number) {
    return fibonacci(n);
  },
};

const io = new BunIo(Bun.stdin.stream());
const channel = new RPCChannel(io, { expose: api });
