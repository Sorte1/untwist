import { spawnSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const tsc = join(here, "node_modules", ".bin", "tsc");

const compilerOptions = {
  strict: true,
  noImplicitAny: false,
  erasableSyntaxOnly: true,
  isolatedModules: true,
  useDefineForClassFields: true,
  module: "esnext",
  moduleResolution: "bundler",
  moduleDetection: "force",
  target: "esnext",
  lib: ["esnext", "dom"],
  jsx: "preserve",
  noEmit: true,
  allowJs: true,
  checkJs: true,
  skipLibCheck: true,
  types: [],
};

function diagnostics(dir) {
  writeFileSync(
    join(dir, "tsconfig.json"),
    JSON.stringify({ compilerOptions, include: ["*.ts", "*.tsx", "*.js", "*.jsx"] }, null, 2),
  );
  const run = spawnSync(tsc, ["-p", dir, "--pretty", "false"], {
    encoding: "utf8",
    maxBuffer: 1 << 30,
  });
  if (run.error) throw run.error;
  const found = new Map();
  for (const line of run.stdout.split("\n")) {
    const match = /^(.+?)\((\d+),(\d+)\): error (TS\d+): (.*)$/.exec(line);
    if (!match) continue;
    const [, file, row, , code, message] = match;
    const name = file.split(/[\\/]/).pop();
    if (!found.has(name)) found.set(name, []);
    found.get(name).push({ row: Number(row), code, message });
  }
  return found;
}

function key(entry) {
  return `${entry.row}:${entry.code}`;
}

function delta(root) {
  const annotated = diagnostics(join(root, "annotated"));
  const plain = existsSync(join(root, "plain")) ? diagnostics(join(root, "plain")) : new Map();
  const added = [];
  for (const [file, entries] of annotated) {
    const before = new Map();
    for (const entry of plain.get(file) ?? []) {
      before.set(key(entry), (before.get(key(entry)) ?? 0) + 1);
    }
    for (const entry of entries) {
      const left = before.get(key(entry)) ?? 0;
      if (left > 0) {
        before.set(key(entry), left - 1);
        continue;
      }
      const text = readFileSync(join(root, "annotated", file), "utf8").split("\n")[entry.row - 1] ?? "";
      added.push(`${file}:${entry.row} ${entry.code} ${entry.message}\n    ${text.trim().slice(0, 200)}`);
    }
  }
  return added;
}

const roots = process.argv.slice(2);
if (roots.length === 0) {
  console.error("usage: node check.mjs <pairs-dir>...   (each holds annotated/ and plain/)");
  process.exit(2);
}
let failed = false;
for (const root of roots.map((path) => resolve(path))) {
  if (!existsSync(join(root, "annotated"))) continue;
  const files = readdirSync(join(root, "annotated")).filter((name) => name !== "tsconfig.json").length;
  const added = delta(root);
  console.log(`${root}: ${files} annotated files, ${added.length} new diagnostics`);
  for (const line of added) console.log(line);
  failed ||= added.length > 0;
}
process.exit(failed ? 1 : 0);
