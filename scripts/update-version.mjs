#!/usr/bin/env node
import { readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const VERSION_PATTERN = /^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/;

function assertValidVersion(version) {
  if (!VERSION_PATTERN.test(version)) {
    throw new Error("版本号必须是 SemVer，例如 0.1.6 或 1.0.0-beta.1。");
  }
}

async function readJson(path) {
  return JSON.parse(await readFile(path, "utf8"));
}

async function writeJson(path, value) {
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`);
}

async function updatePackageJson(root, version) {
  const path = join(root, "package.json");
  const content = await readJson(path);
  content.version = version;
  await writeJson(path, content);
  return "package.json";
}

async function updatePackageLock(root, version) {
  const path = join(root, "package-lock.json");
  const content = await readJson(path);
  content.version = version;
  if (content.packages?.[""]) {
    content.packages[""].version = version;
  }
  await writeJson(path, content);
  return "package-lock.json";
}

async function updateCargoToml(root, version) {
  const path = join(root, "src-tauri", "Cargo.toml");
  const content = await readFile(path, "utf8");
  const updated = content.replace(
    /(\[package\][\s\S]*?\nversion\s*=\s*)"[^"]+"/,
    `$1"${version}"`,
  );

  if (updated === content) {
    throw new Error("未找到 src-tauri/Cargo.toml 的 [package].version。");
  }

  await writeFile(path, updated);
  return "src-tauri/Cargo.toml";
}

async function updateCargoLock(root, version) {
  const path = join(root, "src-tauri", "Cargo.lock");
  const content = await readFile(path, "utf8");
  const updated = content.replace(
    /(\[\[package\]\]\r?\nname = "devdock"\r?\nversion = )"[^"]+"/,
    `$1"${version}"`,
  );

  if (updated === content) {
    throw new Error("未找到 src-tauri/Cargo.lock 中 name = \"devdock\" 的版本号。");
  }

  await writeFile(path, updated);
  return "src-tauri/Cargo.lock";
}

async function updateTauriConfig(root, version) {
  const path = join(root, "src-tauri", "tauri.conf.json");
  const content = await readJson(path);
  content.version = version;
  await writeJson(path, content);
  return "src-tauri/tauri.conf.json";
}

export async function updateProjectVersion(root, version) {
  assertValidVersion(version);

  const changedFiles = [];
  changedFiles.push(await updatePackageJson(root, version));
  changedFiles.push(await updatePackageLock(root, version));
  changedFiles.push(await updateCargoToml(root, version));
  changedFiles.push(await updateCargoLock(root, version));
  changedFiles.push(await updateTauriConfig(root, version));

  return changedFiles;
}

async function main() {
  const version = process.argv[2];
  if (!version) {
    throw new Error("用法：npm run version:set -- 0.1.6");
  }

  const scriptDir = dirname(fileURLToPath(import.meta.url));
  const root = join(scriptDir, "..");
  const changedFiles = await updateProjectVersion(root, version);
  console.log(`版本号已更新为 ${version}`);
  for (const file of changedFiles) {
    console.log(`- ${file}`);
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
}
