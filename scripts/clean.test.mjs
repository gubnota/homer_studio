import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { copyFile, mkdtemp, mkdir, readFile, rm, symlink, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const repository = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const name = JSON.parse(await readFile(path.join(repository, 'package.json'), 'utf8')).name;

async function fixture(t) {
  const root = await mkdtemp(path.join(os.tmpdir(), 'studio-clean-test-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  await mkdir(path.join(root, 'scripts'));
  await copyFile(path.join(repository, 'scripts/clean.mjs'), path.join(root, 'scripts/clean.mjs'));
  await writeFile(path.join(root, 'package.json'), JSON.stringify({ name }));
  return root;
}

async function marker(root, relative) {
  const file = path.join(root, relative, 'marker');
  await mkdir(path.dirname(file), { recursive: true });
  await writeFile(file, 'keep or remove');
  return file;
}

function clean(root, ...args) {
  return spawnSync(process.execPath, [path.join(root, 'scripts/clean.mjs'), ...args], { encoding: 'utf8' });
}

test('default cleanup removes builds but preserves projects and models', async t => {
  const root = await fixture(t);
  const build = await marker(root, 'src-tauri/target');
  const project = await marker(root, 'Example.wavehs/media');
  const model = await marker(root, 'workers/chatterbox/models');
  const preview = clean(root, '--dry-run');
  assert.equal(preview.status, 0, preview.stderr);
  assert.match(preview.stdout, /src-tauri\/target/);
  assert.equal((await readFile(build, 'utf8')).length > 0, true);
  const result = clean(root);
  assert.equal(result.status, 0, result.stderr);
  await assert.rejects(readFile(build));
  assert.equal(await readFile(project, 'utf8'), 'keep or remove');
  assert.equal(await readFile(model, 'utf8'), 'keep or remove');
});

test('refuses a build folder symlink that points outside the repository', async t => {
  const root = await fixture(t);
  const outside = await mkdtemp(path.join(os.tmpdir(), 'studio-clean-outside-'));
  t.after(() => rm(outside, { recursive: true, force: true }));
  const file = await marker(outside, 'media');
  await mkdir(path.join(root, 'src-tauri'));
  await symlink(path.join(outside, 'media'), path.join(root, 'src-tauri/target'));
  const result = clean(root);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Refusing to remove unexpected path/);
  assert.equal(await readFile(file, 'utf8'), 'keep or remove');
});

test('model deletion requires an explicit confirmation flag', async t => {
  const root = await fixture(t);
  const model = await marker(root, 'workers/chatterbox/models');
  const result = clean(root, '--models');
  assert.equal(result.status, 2);
  assert.equal(await readFile(model, 'utf8'), 'keep or remove');
});
