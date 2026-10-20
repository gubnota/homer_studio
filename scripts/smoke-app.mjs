import { spawn, execFileSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { createHash, randomUUID } from "node:crypto";
import { gzipSync } from "node:zlib";
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
for (const resource of ["start_local.py", "worker_protocol.py", "chatterbox/server.py", "chatterbox/original.py", "chatterbox/requirements.txt", "audio/processor.py", "audio/inference_driver.py", "audio/seed_vc.txt", "audio/rvc.txt", "audio/deepfilternet.txt", "audio/resemble_enhance.txt"]) {
  const path = join(dirname(dirname(executablePath)), "Resources", "workers", resource);
  if (!existsSync(path)) throw new Error(`Packaged worker file missing: ${path}`);
}

if (process.argv.includes("--documents")) {
  await smokeDocuments();
  process.exit(0);
}

const smokeDataRoot = mkdtempSync(join(tmpdir(), "homer-studio-smoke-"));
const smokeIdentifier = `com.homer-studio.smoke.${randomUUID()}`;
const child = spawn(executablePath, [], {
  env: {
    ...process.env,
    HOMER_SMOKE_IDENTIFIER: smokeIdentifier,
    HOMER_SMOKE_WEB_DATA: join(smokeDataRoot, "web"),
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
  cleanupSmoke();
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
cleanupSmoke();
console.log("Packaged app remained healthy through the startup smoke window.");

function cleanupSmoke() {
  rmSync(smokeDataRoot, {recursive: true, force: true});
  for (const directory of ["Application Support", "Caches", "WebKit"]) rmSync(join(homedir(), "Library", directory, smokeIdentifier), {recursive: true, force: true});
}

// LaunchServices delivers Opened before Ready on cold document launches. Passing a
// path in argv alone does not exercise that ordering. Use a disposable signed app
// copy so an installed/running user app cannot receive these test documents.
async function smokeDocuments() {
  if (process.platform !== "darwin") throw new Error("Document smoke requires macOS.");
  const root = realpathSync(mkdtempSync(join(tmpdir(), "homer-document-smoke-")));
  const identifier = `com.homer-studio.smoke.${randomUUID()}`;
  const app = join(root, "Document Smoke.app");
  const executable = join(app, "Contents", "MacOS", "homer-studio");
  const data = join(homedir(), "Library", "Application Support", identifier, "wave-studio");
  let launcher;
  const sleep = (ms) => new Promise(resolveSleep => setTimeout(resolveSleep, ms));
  const pids = () => execFileSync("/bin/ps", ["-axo", "pid=,command="], {encoding: "utf8"})
    .split("\n").filter(line => line.trim().endsWith(executable))
    .map(line => Number(line.trim().split(/\s+/)[0]));
  const stop = async () => {
    for (const pid of pids()) { try { process.kill(pid, "SIGTERM"); } catch {} }
    for (let n = 0; n < 20 && pids().length; n++) await sleep(100);
    for (const pid of pids()) { try { process.kill(pid, "SIGKILL"); } catch {} }
    if (launcher && launcher.exitCode === null) launcher.kill("SIGTERM");
  };
  const manifests = () => {
    const dir = join(data, "projects");
    return existsSync(dir) ? readdirSync(dir).filter(name => name.endsWith(".json"))
      .map(name => JSON.parse(readFileSync(join(dir, name), "utf8"))) : [];
  };
  try {
    cpSync(dirname(dirname(dirname(executablePath))), app, {recursive: true});
    execFileSync("/usr/bin/plutil", ["-replace", "CFBundleIdentifier", "-string", identifier, join(app, "Contents", "Info.plist")]);
    execFileSync("/usr/bin/codesign", ["--force", "--deep", "--sign", "-", app], {stdio: "pipe"});

    const sourceId = randomUUID();
    const wav = Buffer.alloc(44 + 4800 * 2 * 4);
    wav.write("RIFF", 0); wav.writeUInt32LE(wav.length - 8, 4); wav.write("WAVEfmt ", 8);
    wav.writeUInt32LE(16, 16); wav.writeUInt16LE(3, 20); wav.writeUInt16LE(2, 22);
    wav.writeUInt32LE(48000, 24); wav.writeUInt32LE(48000 * 8, 28);
    wav.writeUInt16LE(8, 32); wav.writeUInt16LE(32, 34); wav.write("data", 36);
    wav.writeUInt32LE(wav.length - 44, 40);
    for (let frame = 0; frame < 4800; frame++) {
      const sample = Math.sin(frame * Math.PI * 2 * 440 / 48000) * 0.1;
      wav.writeFloatLE(sample, 44 + frame * 8); wav.writeFloatLE(sample, 48 + frame * 8);
    }
    const project = {
      schemaVersion: 1, id: randomUUID(), name: "Document smoke", revision: 0,
      updatedAtMs: Date.now(), sources: [{id: sourceId, name: "Tone", durationMs: 100, channels: 2}],
      timeline: {clips: [{id: randomUUID(), sourceId, name: "Tone", startMs: 0,
        sourceStartMs: 0, sourceEndMs: 100, speed: 1, gainDb: 0, fadeInMs: 0, fadeOutMs: 0}], sfx: [], voices: []}, videos: [],
    };
    const entries = [["project.json", Buffer.from(JSON.stringify(project))],
      ["voices.json", Buffer.from("[]")], [`media/${sourceId}.wav`, wav]];
    const folder = join(root, "Folder project.wavehs");
    mkdirSync(join(folder, "media"), {recursive: true});
    for (const [name, bytes] of entries) writeFileSync(join(folder, name), bytes);
    const count = Buffer.alloc(4); count.writeUInt32LE(entries.length);
    const payload = [count];
    for (const [name, bytes] of entries) {
      const path = Buffer.from(name), length = Buffer.alloc(2), size = Buffer.alloc(8);
      length.writeUInt16LE(path.length); size.writeBigUInt64LE(BigInt(bytes.length));
      payload.push(length, path, size, createHash("sha256").update(bytes).digest(), bytes);
    }
    const archive = join(root, "Portable project.wavehs");
    writeFileSync(archive, Buffer.concat([Buffer.from("WAVEHS02"), gzipSync(Buffer.concat(payload))]));
    const malformed = join(root, "Malformed.wavehs"); writeFileSync(malformed, "invalid bundle");

    for (const [label, document, second] of [["archive", archive, folder], ["folder", folder, archive]]) {
      await stop(); rmSync(data, {recursive: true, force: true});
      const stderr = join(root, `${label}.stderr`);
      launcher = spawn("/usr/bin/open", ["-n", "-F", "-W", "-a", app,
        "--env", `HOMER_SMOKE_IDENTIFIER=${identifier}`,
        "--env", `HOMER_SMOKE_WEB_DATA=${join(root, `web-${label}`)}`,
        "--stderr", stderr, document], {stdio: ["ignore", "pipe", "pipe"]});
      let launchError = "";
      launcher.stderr.on("data", chunk => { launchError += chunk.toString(); });
      launcher.on("error", error => { launchError += error.message; });
      const waitFor = async (test, description) => {
        for (let n = 0; n < 300; n++) {
          if (launcher.exitCode !== null || launcher.signalCode !== null || launchError) {
            throw new Error(`${label}: app exited during ${description}. ${launchError}\n${existsSync(stderr) ? readFileSync(stderr, "utf8") : ""}`);
          }
          if (test()) return;
          await sleep(100);
        }
        throw new Error(`${label}: timed out waiting for ${description}; data: ${existsSync(data) ? readdirSync(data).join(", ") : "missing"}.\n${existsSync(stderr) ? readFileSync(stderr, "utf8") : ""}`);
      };
      await waitFor(() => manifests().length === 1, "cold document import");
      const verifyImports = () => {
        for (const imported of manifests()) {
          const folder=JSON.parse(readFileSync(join(data,'folders',`${imported.id}.json`),'utf8'));
          if (imported.timeline.clips[0].sourceId !== imported.sources[0].id ||
              !readFileSync(join(folder, "media", `${imported.sources[0].id}.wav`)).equals(wav) ||
              (existsSync(join(data,'assets'))&&readdirSync(join(data,'assets')).some(name=>name.endsWith('.wav')))) {
            throw new Error(`${label}: imported project identity or media did not round-trip.`);
          }
        }
      };
      verifyImports();
      execFileSync("/usr/bin/open", ["-a", app, second]);
      await waitFor(() => manifests().length === 2, "running-app document import");
      verifyImports();
      execFileSync("/usr/bin/open", ["-a", app, second]);
      await sleep(1500);
      execFileSync("/usr/bin/open", ["-a", app, malformed]);
      await sleep(1500);
      if (pids().length !== 1 || launcher.exitCode !== null ||
          (existsSync(stderr) && /panicked|Failed to setup app/.test(readFileSync(stderr, "utf8")))) {
        throw new Error(`${label}: repeated or malformed document opening crashed the app.`);
      }
      console.log(`Passed ${label}: cold/running document import, media round-trip, repeated and malformed opens.`);
    }
  } finally {
    await stop();
    rmSync(root, {recursive: true, force: true});
    for (const directory of ["Application Support", "Caches", "WebKit", "Saved Application State"]) {
      rmSync(join(homedir(), "Library", directory, directory === "Saved Application State" ? `${identifier}.savedState` : identifier), {recursive: true, force: true});
    }
  }
}
