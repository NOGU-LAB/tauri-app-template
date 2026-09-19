import { spawn } from "node:child_process";
import process from "node:process";

const port = "8765";
const token = "tauri-local-development-token";
const npmCommand = process.platform === "win32"
  ? { command: process.env.ComSpec || "cmd.exe", args: ["/d", "/s", "/c", "npm.cmd", "run", "tauri", "dev"] }
  : { command: "npm", args: ["run", "tauri", "dev"] };
const children = new Set();
let stopping = false;

function start(command, args, options) {
  const child = spawn(command, args, { stdio: "inherit", ...options });
  children.add(child);
  child.on("error", (error) => {
    console.error(`${command} の起動に失敗しました:`, error.message);
    stop(1);
  });
  child.on("exit", (code, signal) => {
    children.delete(child);
    if (!stopping) {
      if (code !== 0 || signal) {
        console.error(`${command} が終了しました (code=${code}, signal=${signal})`);
      }
      stop(code ?? (signal ? 1 : 0));
    }
  });
  return child;
}

function stop(code = 0) {
  if (stopping) return;
  stopping = true;
  process.exitCode = code;
  for (const child of children) child.kill("SIGTERM");
  setTimeout(() => process.exit(code), 500);
}

const sharedEnv = {
  ...process.env,
  DEV_PORT: port,
  DEV_BACKEND_TOKEN: token,
};

start("air", ["-c", ".air.toml"], {
  cwd: new URL("../backend/", import.meta.url),
  env: sharedEnv,
});
start(npmCommand.command, npmCommand.args, {
  env: {
    ...process.env,
    TAURI_EXTERNAL_BACKEND_PORT: port,
    TAURI_EXTERNAL_BACKEND_TOKEN: token,
  },
});

process.on("SIGINT", () => stop(0));
process.on("SIGTERM", () => stop(0));
