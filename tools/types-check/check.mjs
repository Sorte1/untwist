import { spawn } from "node:child_process";
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

const BATCH = 200;
const RUNS = Math.max(1, Number(process.env.TYPES_CHECK_RUNS ?? 3));
const SHOWN = 400;
const codes = new Map();

function sources(dir) {
  return existsSync(dir)
    ? readdirSync(dir).filter((name) => !name.startsWith("tsconfig") && /\.(d\.ts|ts|tsx|js|jsx)$/.test(name))
    : [];
}

async function diagnostics(dir, names) {
  const found = new Map();
  if (names.length === 0) return found;
  const config = join(dir, "tsconfig.batch.json");
  writeFileSync(config, JSON.stringify({ compilerOptions, files: names }, null, 2));
  const child = spawn(tsc, ["-p", config, "--pretty", "false"], { stdio: ["ignore", "pipe", "inherit"] });
  const LINE_CAP = 2000;
  let line = "";
  let full = false;
  const take = (text) => {
    const match = /^(.+?)\((\d+),(\d+)\): error (TS\d+): (.*)$/.exec(text);
    if (!match) return;
    const [, file, row, , code, message] = match;
    const name = stem(file.split(/[\\/]/).pop());
    if (!found.has(name)) found.set(name, []);
    found.get(name).push({ row: Number(row), code, message: message.slice(0, 300) });
  };
  child.stdout.setEncoding("utf8");
  for await (const chunk of child.stdout) {
    let from = 0;
    for (;;) {
      const end = chunk.indexOf("\n", from);
      const piece = end === -1 ? chunk.slice(from) : chunk.slice(from, end);
      if (!full) {
        line += piece.slice(0, LINE_CAP - line.length);
        full = line.length >= LINE_CAP;
      }
      if (end === -1) break;
      take(line);
      line = "";
      full = false;
      from = end + 1;
    }
  }
  take(line);
  await new Promise((done) => child.on("close", done));
  return found;
}

function stem(name) {
  return name.replace(/\.(d\.ts|ts|tsx|js|jsx|mjs|cjs|mts|cts)$/, "");
}

function key(entry) {
  return `${entry.row}:${entry.code}`;
}

function tally(entries) {
  const counts = new Map();
  for (const entry of entries) {
    const held = counts.get(key(entry));
    if (held) held.count += 1;
    else counts.set(key(entry), { count: 1, entry });
  }
  return counts;
}

async function runs(dir, names) {
  const all = [];
  for (let run = 0; run < RUNS; run += 1) {
    const counted = new Map();
    for (const [file, entries] of await diagnostics(dir, names)) counted.set(file, tally(entries));
    all.push(counted);
  }
  return all;
}

async function delta(root) {
  const annotatedDir = join(root, "annotated");
  const plainDir = join(root, PLAIN);
  const annotatedNames = sources(annotatedDir);
  const plainByStem = new Map(sources(plainDir).map((name) => [stem(name), name]));
  const added = [];
  const texts = new Map();
  const report = (file, entry) => {
    if (added.length >= SHOWN) {
      added.push(null);
      return;
    }
    const real = annotatedNames.find((name) => stem(name) === file) ?? file;
    if (!texts.has(real)) texts.set(real, readFileSync(join(annotatedDir, real), "utf8").split("\n"));
    const text = texts.get(real)[entry.row - 1] ?? "";
    added.push(`${file}:${entry.row} ${entry.code} ${entry.message}\n    ${text.trim().slice(0, 200)}`);
    codes.set(entry.code, (codes.get(entry.code) ?? 0) + 1);
  };
  for (let at = 0; at < annotatedNames.length; at += BATCH) {
    texts.clear();
    const batch = annotatedNames.slice(at, at + BATCH);
    const annotated = await runs(annotatedDir, batch);
    const plainNames = batch.map((name) => plainByStem.get(stem(name))).filter(Boolean);
    const plain = await runs(plainDir, plainNames);
    const worst = new Map();
    for (let run = 0; run < RUNS; run += 1) {
      for (const [file, counts] of annotated[run]) {
        const before = plain[run].get(file) ?? new Map();
        for (const [at, { count, entry }] of counts) {
          const extra = count - (before.get(at)?.count ?? 0);
          const id = `${file}\u0000${at}`;
          if (extra > 0 && (worst.get(id)?.extra ?? 0) < extra) worst.set(id, { file, entry, extra });
        }
      }
    }
    for (const { file, entry, extra } of worst.values()) {
      for (let left = 0; left < extra; left += 1) report(file, entry);
    }
  }
  return added;
}

const args = process.argv.slice(2);
const against = args.indexOf("--against-js");
const PLAIN = against === -1 ? "plain" : "plain-js";
const roots = args.filter((arg) => arg !== "--against-js");
if (roots.length === 0) {
  console.error("usage: node check.mjs [--against-js] <pairs-dir>...   (each holds annotated/, plain/ and plain-js/; TYPES_CHECK_RUNS paired tsc runs, default 3)");
  process.exit(2);
}
let failed = false;
for (const root of roots.map((path) => resolve(path))) {
  if (!existsSync(join(root, "annotated"))) continue;
  const files = sources(join(root, "annotated")).length;
  const added = await delta(root);
  console.log(`${root}: ${files} annotated files, ${added.length} new diagnostics`);
  for (const line of added.filter(Boolean)) console.log(line);
  failed ||= added.length > 0;
}
if (codes.size > 0) console.log([...codes].sort((a, b) => b[1] - a[1]).map(([code, count]) => `${code} ${count}`).join("\n"));
process.exit(failed ? 1 : 0);
