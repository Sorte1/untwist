import { existsSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { form, walk } from "../libcorpus/files.mjs";
import { entries } from "../libcorpus/libs.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const CORPUS = join(HERE, "../libcorpus");
const EXCLUDES_FILE = join(HERE, "wide-excludes.json");
const MAX_FILE_BYTES = 4 * 1024 * 1024;
const EXCLUDES = existsSync(EXCLUDES_FILE) ? JSON.parse(readFileSync(EXCLUDES_FILE, "utf8")) : {};
const KEPT_FORMS = new Set(["cjs", "esm", "umd", "min", "script"]);

export function packageDir(entry) {
  return join(CORPUS, entry.root, "node_modules", entry.alias);
}

export function files(entry, { include, exclude }) {
  const dir = packageDir(entry);
  const skipped = new Set(EXCLUDES[entry.alias] ?? []);
  const out = [];
  for (const [abs, rel] of walk(dir, dir, entry.name)) {
    if (skipped.has(rel) || (exclude && exclude.test(rel))) continue;
    if (statSync(abs).size > MAX_FILE_BYTES) continue;
    const kind = form(rel, readFileSync(abs, "utf8"));
    if (KEPT_FORMS.has(kind) && include(rel, kind)) out.push(rel);
  }
  return out.sort();
}

function specifier(entry, rel) {
  const manifest = JSON.parse(readFileSync(join(packageDir(entry), "package.json"), "utf8"));
  return manifest.exports ? join(packageDir(entry), rel) : `${entry.alias}/${rel}`;
}

export function entrySource(entry, rels, { main }) {
  const lines = [];
  const keep = [];
  if (main) {
    lines.push(`import * as m0 from ${JSON.stringify(entry.alias)};`);
    keep.push("m0");
  }
  rels.forEach((rel, index) => {
    const name = `m${index + 1}`;
    lines.push(`import * as ${name} from ${JSON.stringify(specifier(entry, rel))};`);
    keep.push(name);
  });
  lines.push(`globalThis.__untwistLibrary = [${keep.join(", ")}];`);
  return `${lines.join("\n")}\n`;
}

const DIST = (rel, kind) => kind === "min" || kind === "umd" || /^(dist|build|umd|bundles|lib)\//.test(rel) || !rel.includes("/");

export function wide({ id, name, label = name, include = DIST, exclude = null, main = true, ...rest }) {
  const versions = entries().filter((entry) => entry.name === name);
  if (!versions.length) throw new Error(`no corpus versions for ${name}`);
  const byDir = new Map(versions.map((entry) => [entry.alias, entry]));
  return {
    id,
    label,
    versions: versions.map((entry) => ({ dir: entry.alias, version: entry.version, root: join("tools/libcorpus", entry.root) })),
    entry(dir) {
      const entry = byDir.get(dir);
      const mainExcluded = (EXCLUDES[entry.alias] ?? []).includes("<main>");
      return entrySource(entry, files(entry, { include, exclude }), { main: main && !mainExcluded });
    },
    module(rel) {
      const match = /^(.*)\.(c|m)?js$/.exec(rel);
      return match ? match[1] : null;
    },
    ...rest,
  };
}

export const ALL = () => true;
