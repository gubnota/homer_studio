#!/usr/bin/env node
import { lstat, readFile, readdir, realpath, rm } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const choices = {
  unipuni: {
    build: ['src-tauri/target', 'dist', 'src-tauri/gen/schemas', 'node_modules/.vite'],
    runtimes: [],
    models: [],
  },
  'homer-studio': {
    build: ['src-tauri/target', 'target', 'dist', 'coverage', 'test-results', 'playwright-report', 'node_modules/.vite', 'workers/chatterbox/.cache', 'workers/sfx/.cache'],
    runtimes: ['workers/chatterbox/.venv', 'workers/sfx/.venv'],
    models: ['workers/chatterbox/models', 'workers/sfx/models'],
  },
};

const args = new Set(process.argv.slice(2));
const valid = new Set(['--dry-run', '--runtimes', '--models', '--yes', '--help']);
if ([...args].some(arg => !valid.has(arg))) {
  console.error('Unknown option. Run npm run clean -- --help.');
  process.exit(2);
}
if (args.has('--help')) {
  console.log('Usage: npm run clean -- [--dry-run] [--runtimes] [--models --yes]');
  console.log('Default removes only build output. --runtimes removes local Python environments.');
  console.log('--models removes downloaded repository-local weights and requires --yes.');
  process.exit(0);
}
if (args.has('--models') && !args.has('--yes') && !args.has('--dry-run')) {
  console.error('Model deletion requires both --models and --yes. Preview with --models --dry-run.');
  process.exit(2);
}

const manifest = JSON.parse(await readFile(path.join(root, 'package.json'), 'utf8'));
const config = choices[manifest.name];
if (!config) throw new Error(`Unsupported repository: ${manifest.name}`);
const rootReal = await realpath(root);
const selected = [
  ...config.build,
  ...(args.has('--runtimes') ? config.runtimes : []),
  ...(args.has('--models') ? config.models : []),
];

async function bytesAt(file) {
  const info = await lstat(file);
  if (!info.isDirectory() || info.isSymbolicLink()) return info.size;
  let total = 0;
  for (const name of await readdir(file)) total += await bytesAt(path.join(file, name));
  return total;
}

function insideRoot(file) {
  const relative = path.relative(rootReal, file);
  return relative && relative !== '..' && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative);
}

let total = 0;
for (const relative of selected) {
  const target = path.join(root, relative);
  let info;
  try { info = await lstat(target); }
  catch (error) {
    if (error.code === 'ENOENT') continue;
    throw error;
  }
  if (info.isSymbolicLink() || !info.isDirectory() || !insideRoot(await realpath(target))) {
    throw new Error(`Refusing to remove unexpected path: ${relative}`);
  }
  const bytes = await bytesAt(target);
  total += bytes;
  console.log(`${args.has('--dry-run') ? 'Would remove' : 'Removing'} ${relative} (${(bytes / 1024 ** 3).toFixed(2)} GiB)`);
  if (!args.has('--dry-run')) await rm(target, { recursive: true });
}
console.log(`${args.has('--dry-run') ? 'Reclaimable' : 'Removed'}: ${(total / 1024 ** 3).toFixed(2)} GiB`);
console.log('Project folders, app data, Git history, and installed apps were not touched.');
