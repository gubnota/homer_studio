import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const bundleRoot = join(repositoryRoot, "src-tauri", "target", "release", "bundle");
const appPath = join(bundleRoot, "macos", "Homer Studio.app");
const executablePath = join(appPath, "Contents", "MacOS", "homer-studio");
const packageVersion = JSON.parse(
  readFileSync(join(repositoryRoot, "package.json"), "utf8"),
).version;
const dmgPath = join(bundleRoot, "dmg", `Homer Studio_${packageVersion}_aarch64.dmg`);
const zipPath = join(bundleRoot, "macos", `Homer Studio_${packageVersion}_aarch64.zip`);
const argumentsSet = new Set(process.argv.slice(2));

function requireFile(path, label) {
  if (!existsSync(path)) {
    throw new Error(`${label} was not created: ${path}`);
  }
}

function run(command, args) {
  return execFileSync(command, args, { encoding: "utf8" }).trim();
}

function verifyVersionTag() {
  const tagIndex = process.argv.indexOf("--tag");
  if (tagIndex === -1) return;

  const suppliedTag = process.argv[tagIndex + 1];
  if (suppliedTag !== `v${packageVersion}`) {
    throw new Error(
      `Release tag ${suppliedTag ?? "<missing>"} does not match package version v${packageVersion}.`,
    );
  }
}

verifyVersionTag();
if (argumentsSet.has("--version-only")) {
  console.log(`Verified release tag for package version ${packageVersion}.`);
  process.exit(0);
}
requireFile(executablePath, "Application executable");
const iconName = run("/usr/libexec/PlistBuddy", ["-c", "Print :CFBundleIconFile", join(appPath, "Contents", "Info.plist")]);
if (!iconName) throw new Error("The app bundle has no icon declaration.");
requireFile(join(appPath, "Contents", "Resources", iconName.endsWith(".icns") ? iconName : `${iconName}.icns`), "Application icon");
for (const resource of [
  "workers/start_local.py",
  "workers/worker_protocol.py",
  "workers/chatterbox/server.py",
  "workers/chatterbox/original.py",
  "workers/chatterbox/requirements.txt",
]) {
  requireFile(join(appPath, "Contents", "Resources", resource), `Bundled worker ${resource}`);
}

const architecture = run("/usr/bin/file", [executablePath]);
if (!architecture.includes("arm64")) {
  throw new Error(`Application executable is not arm64: ${architecture}`);
}

run("/usr/bin/codesign", ["--verify", "--deep", "--strict", appPath]);

if (argumentsSet.has("--create-zip")) {
  run("/usr/bin/ditto", [
    "-c",
    "-k",
    "--sequesterRsrc",
    "--keepParent",
    appPath,
    zipPath,
  ]);
}

if (argumentsSet.has("--package")) {
  requireFile(dmgPath, "Disk image");
  requireFile(zipPath, "ZIP archive");
}

console.log(`Verified arm64 app and ad-hoc signature: ${appPath}`);
if (argumentsSet.has("--package")) {
  console.log(`Verified release packages: ${dmgPath} and ${zipPath}`);
}
