import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { dirname, extname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { entries } from "./libs.mjs";
import { ROOTS, packageName } from "./roots.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT = process.env.UNTWIST_LIBCORPUS_OUT ?? join(process.env.HOME, ".cache/untwist-libcorpus");
const SKIP_DIRS = new Set([
  "test", "tests", "__tests__", "spec", "specs", "example", "examples", "docs", "doc", "bench", "benchmark",
  "benchmarks", "scripts", "bin", "typings", "types", "node_modules", "coverage", "fixtures", "perf", "demo",
  "demos", "website", "editor", "manual", "playground", "devtools", "flow-typed", "__mocks__",
]);
const KEEP_EXAMPLES = new Map([["three", "examples/jsm"]]);
const SKIP_UNDER = new Map([["three", ["examples/jsm/libs"]]]);
const SOURCE_EXT = new Set([".js", ".mjs", ".cjs"]);

function resolveDep(LOCK, fromPath, dep) {
  let base = fromPath;
  for (;;) {
    const candidate = base ? `${base}/node_modules/${dep}` : `node_modules/${dep}`;
    if (LOCK[candidate]) return candidate;
    if (!base) return null;
    const cut = base.lastIndexOf("/node_modules/");
    base = cut < 0 ? "" : base.slice(0, cut);
  }
}

function closure(roots) {
  const seen = new Map();
  const queue = roots.map((root) => [`${root.root}:${root.path}`, root]);
  while (queue.length) {
    const [key, root] = queue.shift();
    if (seen.has(key)) continue;
    seen.set(key, root);
    const lock = ROOTS.find((candidate) => candidate.root === root.root).lock;
    const path = key.slice(root.root.length + 1);
    for (const dep of Object.keys(lock[path].dependencies ?? {})) {
      const next = resolveDep(lock, path, dep);
      if (next) queue.push([`${root.root}:${next}`, root]);
    }
  }
  return seen;
}

function form(file, text) {
  if (/\.min\.(c|m)?js$/.test(file) || /[.-]min[.-]/.test(file)) return "min";
  const head = text.slice(0, 4000);
  if (/typeof\s+define\s*===?\s*['"]function['"]\s*&&\s*define\.amd/.test(head) && /typeof\s+(exports|module)\b/.test(head)) return "umd";
  if (file.endsWith(".mjs") || /^\s*(import\s*[\w{*'"]|export\s+[\w{*])/m.test(text)) return "esm";
  if (file.endsWith(".cjs") || /\brequire\s*\(|\bmodule\.exports\b|\bexports\.\w+\s*=/.test(text)) return "cjs";
  return "script";
}

function* walk(root, dir, name) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const abs = join(dir, entry.name);
    const rel = relative(root, abs);
    if (entry.isDirectory()) {
      if ((SKIP_UNDER.get(name) ?? []).some((prefix) => rel === prefix)) continue;
      const keep = KEEP_EXAMPLES.get(name);
      if (keep && (keep === rel || keep.startsWith(`${rel}/`) || rel.startsWith(`${keep}/`))) {
        yield* walk(root, abs, name);
        continue;
      }
      if (SKIP_DIRS.has(entry.name.toLowerCase())) continue;
      yield* walk(root, abs, name);
    } else if (entry.isFile() && SOURCE_EXT.has(extname(entry.name)) && !entry.name.endsWith(".d.ts")) {
      yield [abs, rel];
    }
  }
}

const roots = entries().map((entry) => ({ ...entry, path: `node_modules/${entry.alias}` }));
const packages = closure(roots);
const rows = [];
const byHash = new Map();
for (const [key, root] of packages) {
  const path = key.slice(root.root.length + 1);
  const info = ROOTS.find((candidate) => candidate.root === root.root).lock[path];
  const name = packageName(info, path);
  const dir = join(HERE, root.root, path);
  const direct = path === root.path;
  for (const [abs, rel] of walk(dir, dir, name)) {
    const size = statSync(abs).size;
    if (size === 0 || size > 16 * 1024 * 1024) continue;
    const text = readFileSync(abs, "utf8");
    const hash = createHash("sha256").update(text).digest("hex");
    const kind = form(rel, text);
    const row = { lib: name, version: info.version, group: root.group, via: direct ? "" : `${root.name}@${root.version}`, form: kind, file: rel, bytes: size, sha256: hash, path: relative(HERE, abs) };
    if (byHash.has(hash)) {
      byHash.get(hash).push(`${name}@${info.version}/${rel}`);
      continue;
    }
    byHash.set(hash, []);
    rows.push(row);
  }
}
mkdirSync(OUT, { recursive: true });
const header = ["lib", "version", "group", "via", "form", "file", "bytes", "sha256", "path"];
const lines = rows.map((row) => header.map((key) => row[key]).join("\t"));
writeFileSync(join(OUT, "corpus.tsv"), `${header.join("\t")}\n${lines.join("\n")}\n`);
const dups = [...byHash.entries()].filter(([, extra]) => extra.length).map(([hash, extra]) => `${hash}\t${extra.join(" ")}`);
writeFileSync(join(OUT, "corpus-duplicates.tsv"), `${dups.join("\n")}\n`);
const summary = {};
for (const row of rows) {
  const key = `${row.lib}@${row.version}`;
  summary[key] ??= { files: 0, bytes: 0, forms: {} };
  summary[key].files += 1;
  summary[key].bytes += row.bytes;
  summary[key].forms[row.form] = (summary[key].forms[row.form] ?? 0) + 1;
}
writeFileSync(join(OUT, "corpus-summary.json"), `${JSON.stringify(summary, null, 1)}\n`);
console.log(`${packages.size} packages, ${rows.length} unique files, ${dups.length} hashes seen more than once -> ${OUT}`);
