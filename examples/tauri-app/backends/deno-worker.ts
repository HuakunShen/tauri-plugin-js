import { expose, stdioJsonTransport } from "npm:kkrpc@^2.1.0/deno";
import process from "node:process";
import type { BackendAPI } from "./shared-api.ts";

function fibonacci(n: number): number {
  if (n <= 1) return n;
  return fibonacci(n - 1) + fibonacci(n - 2);
}

const api: BackendAPI = {
  async add(a: number, b: number) {
    return a + b;
  },
  async echo(message: string) {
    return `[deno] ${message}`;
  },
  async getSystemInfo() {
    return {
      runtime: "deno",
      pid: Deno.pid,
      platform: Deno.build.os,
      arch: Deno.build.arch,
    };
  },
  async fibonacci(n: number) {
    return fibonacci(n);
  },
};

expose(api, stdioJsonTransport({ readable: process.stdin, writable: process.stdout }));
