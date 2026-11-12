import { execFileSync, spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, extname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const HERE = dirname(fileURLToPath(import.meta.url));
export const REPO = resolve(HERE, "../..");
export const OUT = join(REPO, "fixtures/bundle-matrix");
export const WORK = join(HERE, ".work");
export const APPS = join(HERE, "apps");
export const require = createRequire(join(HERE, "package.json"));

export function version(name) {
  let dir = dirname(require.resolve(name));
  while (!existsSync(join(dir, "package.json")) || JSON.parse(readFileSync(join(dir, "package.json"), "utf8")).name === undefined) dir = dirname(dir);
  return JSON.parse(readFileSync(join(dir, "package.json"), "utf8")).version;
}
export const tool = (name, alias = name) => `${name}@${version(alias)}`;

export function filesUnder(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir, { withFileTypes: true })
    .flatMap((d) => (d.isDirectory() ? filesUnder(join(dir, d.name)) : [join(dir, d.name)]))
    .sort();
}

export const jsFiles = (dir) => filesUnder(dir).filter((f) => [".js", ".mjs", ".cjs"].includes(extname(f)));

export function fresh(dir) {
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  return dir;
}

export function copyTree(from, to) {
  for (const file of filesUnder(from)) {
    const target = join(to, relative(from, file));
    mkdirSync(dirname(target), { recursive: true });
    copyFileSync(file, target);
  }
}

export function runCell(dir, entries, loader) {
  const out = spawnSync(process.execPath, ["--no-warnings", join(HERE, "run.mjs"), "--loader", loader, ...entries.map((e) => join(dir, e))], {
    cwd: dir, encoding: "utf8", timeout: 20000,
  });
  return { exit: out.status, stdout: out.stdout ?? "", stderr: out.stderr ?? "" };
}

export function sizeOf(dir) {
  return filesUnder(dir).reduce((sum, f) => sum + statSync(f).size, 0);
}

export function bun() {
  try {
    return execFileSync("bun", ["--version"], { encoding: "utf8" }).trim();
  } catch {
    return null;
  }
}

export function writeText(file, text) {
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, text);
}
