export interface BackendAPI {
  add(a: number, b: number): Promise<number>;
  echo(message: string): Promise<string>;
  getSystemInfo(): Promise<{
    runtime: string;
    pid: number;
    platform: string;
    arch: string;
  }>;
  fibonacci(n: number): Promise<number>;
}
