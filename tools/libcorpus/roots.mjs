import { readFileSync, realpathSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { entries } from "./libs.mjs";

export const HERE = dirname(fileURLToPath(import.meta.url));

export const ROOTS = [...new Set(entries().map((entry) => entry.root))].map((root) => {
  const dir = join(HERE, root);
  const lock = JSON.parse(readFileSync(join(dir, "package-lock.json"), "utf8")).packages;
  const keys = Object.keys(lock).filter((key) => key).sort((a, b) => b.length - a.length);
  return { root, dir, lock, keys };
});

export const packageName = (info, key) => info.name ?? key.slice(key.lastIndexOf("node_modules/") + 13);

export function owner(file) {
  let abs;
  try {
    abs = realpathSync(file);
  } catch {
    return null;
  }
  const root = ROOTS.filter((candidate) => candidate.root !== ".").find((candidate) => abs.startsWith(`${candidate.dir}/`))
    ?? ROOTS.find((candidate) => candidate.root === ".");
  const rel = relative(root.dir, abs);
  const key = root.keys.find((candidate) => rel.startsWith(`${candidate}/`));
  if (!key) return null;
  const info = root.lock[key];
  return { lib: packageName(info, key), version: info.version, file: rel.slice(key.length + 1) };
}
