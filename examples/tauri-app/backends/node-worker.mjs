import { RPCChannel, NodeIo } from "kkrpc";

function fibonacci(n) {
  if (n <= 1) return n;
  return fibonacci(n - 1) + fibonacci(n - 2);
}

const api = {
  async add(a, b) {
    return a + b;
  },
  async echo(message) {
    return `[node] ${message}`;
  },
  async getSystemInfo() {
    return {
      runtime: "node",
      pid: process.pid,
      platform: process.platform,
      arch: process.arch,
    };
  },
  async fibonacci(n) {
    return fibonacci(n);
  },
};

const io = new NodeIo(process.stdin, process.stdout);
const channel = new RPCChannel(io, { expose: api });
