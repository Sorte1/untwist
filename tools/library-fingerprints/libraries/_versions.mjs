import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

export const MODULES = join(dirname(fileURLToPath(import.meta.url)), "..", "node_modules");

export function aliasVersions(prefix) {
  const pattern = new RegExp(`^${prefix.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}-(\\d+)\\.(\\d+)$`);
  return readdirSync(MODULES)
    .map((dir) => [dir, pattern.exec(dir)])
    .filter(([, match]) => match)
    .sort(([, a], [, b]) => Number(a[1]) - Number(b[1]) || Number(a[2]) - Number(b[2]))
    .map(([dir]) => ({ dir, version: JSON.parse(readFileSync(join(MODULES, dir, "package.json"), "utf8")).version }));
}

export function filesIn(dir, folder, pattern) {
  try {
    return readdirSync(join(MODULES, dir, folder)).filter((name) => pattern.test(name)).sort();
  } catch {
    return [];
  }
}
