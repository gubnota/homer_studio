import { spawn } from "node:child_process";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const executablePath = join(
  repositoryRoot,
  "src-tauri",
  "target",
  "release",
  "bundle",
  "macos",
  "Homer Studio.app",
  "Contents",
  "MacOS",
  "homer-studio",
);

if (!existsSync(executablePath)) {
  throw new Error(`Build the app before running the smoke test: ${executablePath}`);
}

const smokeDataRoot = mkdtempSync(join(tmpdir(), "homer-studio-smoke-"));
const child = spawn(executablePath, [], {
  env: {
    ...process.env,
    XDG_CONFIG_HOME: join(smokeDataRoot, "config"),
    XDG_DATA_HOME: join(smokeDataRoot, "data"),
  },
  stdio: ["ignore", "pipe", "pipe"],
});

let stderr = "";
child.stderr.on("data", (chunk) => {
  if (stderr.length < 8_000) stderr += chunk.toString();
});

const exitResult = new Promise((resolveExit) => {
  child.once("exit", (code, signal) => resolveExit({ code, signal }));
});

const earlyExit = await Promise.race([
  exitResult,
  new Promise((resolveAlive) => setTimeout(() => resolveAlive(null), 4_000)),
]);

if (earlyExit) {
  rmSync(smokeDataRoot, { recursive: true, force: true });
  throw new Error(
    `Packaged app exited during startup (${JSON.stringify(earlyExit)}).\n${stderr}`,
  );
}

child.kill("SIGTERM");
await Promise.race([
  exitResult,
  new Promise((resolveTimeout) => setTimeout(resolveTimeout, 2_000)),
]);
if (child.exitCode === null) child.kill("SIGKILL");
rmSync(smokeDataRoot, { recursive: true, force: true });
console.log("Packaged app remained healthy through the startup smoke window.");
