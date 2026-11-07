import { createHash } from "node:crypto";
import { execFile, execFileSync } from "node:child_process";
import { mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { cover } from "./versions-cover.mjs";
import { VERSIONS, cellName, obfuscatorOptions } from "./versions-plan.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "../..");
const OUT = join(REPO, "fixtures/obfuscator-versions");
const SOURCES = join(HERE, "sources");
const WORK = join(HERE, "node_modules/.work-versions");
const RUNNER = join(REPO, "tools/unpack-run.mjs");
const WORKER = join(HERE, "versions-worker.cjs");
const require = createRequire(join(HERE, "package.json"));
const acorn = require("acorn");
const UglifyJS = require("uglify-js");
const { minify: terser } = require("terser-5");
const webpack = require("webpack");

const PROGRAMS = ["empty-log", "greeting", "functions", "objects", "classes", "control", "strings", "mixed"];
const COVER_SEED = 2026;
const PARALLEL = 6;

const sha256 = (text) => createHash("sha256").update(text).digest("hex");
const ATTEMPTS = 4;
const seedOf = (cell, program, attempt) => parseInt(sha256(attempt === 0 ? `${cell}/${program}` : `${cell}/${program}/${attempt}`).slice(0, 8), 16) % 1_000_000_000;
const versionOf = (name) => JSON.parse(readFileSync(require.resolve(`${name}/package.json`), "utf8")).version;

function runNode(file) {
  return execFileSync(process.execPath, ["--no-warnings", RUNNER, "--loader", "node", file], { cwd: dirname(file), encoding: "utf8", timeout: 30000 });
}

function obfuscateAll(name, jobs) {
  return new Promise((done, fail) => {
    const child = execFile(process.execPath, [WORKER, name], { maxBuffer: 1 << 30 }, (error, stdout) => {
      if (error) fail(new Error(`${name}: ${error.message}`));
      else done(JSON.parse(stdout));
    });
    child.stdin.end(JSON.stringify(jobs));
  });
}

async function inBatches(items, work) {
  const out = [];
  for (let i = 0; i < items.length; i += PARALLEL) out.push(...(await Promise.all(items.slice(i, i + PARALLEL).map(work))));
  return out;
}

function externals(context, request, callback) {
  if (request.startsWith("node:")) callback(null, `commonjs ${request}`);
  else callback();
}

function bundle(code, name, verbatim) {
  const entry = join(WORK, `${name}.entry.js`);
  writeFileSync(entry, code);
  const config = {
    context: WORK,
    entry,
    target: "node",
    node: false,
    mode: "none",
    devtool: false,
    externals: [externals],
    module: verbatim ? { noParse: /\.entry\.js$/ } : {},
    optimization: { minimize: false },
    output: { path: WORK, filename: `${name}.bundle.js`, hashFunction: "sha256" },
  };
  return new Promise((done, fail) => {
    webpack(config, (error, stats) => {
      if (error || stats.hasErrors()) fail(error || new Error(stats.toString("errors-only")));
      else done(readFileSync(join(WORK, `${name}.bundle.js`), "utf8"));
    });
  });
}

async function minify(code, minifier) {
  if (minifier === "none") return code;
  if (minifier === "uglify") {
    const result = UglifyJS.minify(code);
    if (result.error) throw new Error(`uglify-js: ${result.error.message} at ${result.error.line}:${result.error.col}`);
    return result.code;
  }
  return (await terser(code)).code;
}

async function contained(code, container, name) {
  if (container === "iife") return `(function () {\n${code}\n})();\n`;
  if (container === "wp4-module") return bundle(code, name, true);
  return code;
}

function parses(code) {
  try {
    acorn.parse(code, { ecmaVersion: "latest", sourceType: "script" });
    return true;
  } catch {
    return false;
  }
}

async function probe(texts, bundles) {
  const allowed = {};
  await inBatches(VERSIONS, async (name) => {
    const jobs = [];
    for (const program of PROGRAMS) {
      jobs.push({ id: `${program}/top`, code: texts[program], options: { seed: 1 } });
      if (bundles[program]) jobs.push({ id: `${program}/bundle`, code: bundles[program], options: { seed: 1 } });
    }
    const results = await obfuscateAll(name, jobs);
    allowed[versionOf(name)] = Object.fromEntries(results.map((r) => [r.id, r.code !== undefined && parses(r.code)]));
  });
  return allowed;
}

function permitted(allowed, bundles) {
  return (version, program, container) => {
    if (container === "wp4-bundle") return Boolean(bundles[program]) && allowed[version][`${program}/bundle`];
    if (container === "wp4-module") return Boolean(bundles[program]) && allowed[version][`${program}/top`];
    return allowed[version][`${program}/top`];
  };
}

async function plainCheck(rows, texts, bundles, expected) {
  const seen = new Set();
  for (const row of rows) {
    const id = `${row.program}--${row.container}--${row.minifier}`;
    if (seen.has(id)) continue;
    seen.add(id);
    const base = row.container === "wp4-bundle" ? bundles[row.program] : texts[row.program];
    const code = await minify(await contained(base, row.container, `plain-${id}`), row.minifier);
    const file = join(WORK, `plain-${id}.js`);
    writeFileSync(file, code);
    if (runNode(file) !== expected[row.program]) throw new Error(`the unobfuscated ${id} build prints other than ${row.program}.js`);
  }
}

function judgement(row) {
  return row.extra === "self" ? "residue-only:hostile" : "node";
}

async function build(row, obfuscated, input) {
  const code = `${await minify(await contained(obfuscated, row.container, row.file), row.minifier).catch((error) => {
    throw new Error(`${row.file}: ${error.message}`);
  })}\n`;
  if (code.includes(REPO) || code.includes(HERE)) throw new Error(`${row.file} embeds a local path`);
  if (!parses(code)) throw new Error(`${row.file} does not parse`);
  if (!gainsArray(input, code)) throw new Error(`${row.file} keeps no string array after ${row.minifier}`);
  return code;
}

function stringArrays(code) {
  const found = new Set();
  const visit = (node) => {
    if (Array.isArray(node)) return node.forEach(visit);
    if (!node || typeof node.type !== "string") return;
    const bound = node.type === "VariableDeclarator" ? node.init : node.type === "AssignmentExpression" ? node.right : null;
    if (bound && bound.type === "ArrayExpression" && bound.elements.length > 0 && bound.elements.every((e) => e && e.type === "Literal" && typeof e.value === "string")) {
      found.add(JSON.stringify(bound.elements.map((e) => e.value)));
    }
    for (const value of Object.values(node)) if (value && typeof value === "object") visit(value);
  };
  visit(acorn.parse(code, { ecmaVersion: "latest", sourceType: "script" }));
  return found;
}

function gainsArray(input, code) {
  const before = stringArrays(input);
  return [...stringArrays(code)].some((array) => !before.has(array));
}

function judged(input, result) {
  if (result.code === undefined) return result;
  if (!parses(result.code)) return { id: result.id, error: "the obfuscated output does not parse" };
  if (!gainsArray(input, result.code)) return { id: result.id, error: "the obfuscated output has no string array" };
  return result;
}

function attempt(row, number) {
  const seed = seedOf(row.cell, row.program, number);
  return { ...row, seed, options: obfuscatorOptions(row.version, row, seed) };
}

async function obfuscateRows(rows, texts, bundles) {
  const byVersion = new Map();
  for (const row of rows) {
    if (!byVersion.has(row.package)) byVersion.set(row.package, []);
    byVersion.get(row.package).push(row);
  }
  const out = new Map();
  await inBatches([...byVersion.keys()], async (name) => {
    const input = (row) => (row.container === "wp4-bundle" ? bundles[row.program] : texts[row.program]);
    const jobs = byVersion.get(name).map((row) => ({ id: row.file, code: input(row), options: row.options }));
    const rowOf = new Map(byVersion.get(name).map((row) => [row.file, row]));
    for (const result of await obfuscateAll(name, jobs)) {
      const row = rowOf.get(result.id);
      out.set(result.id, { ...judged(input(row), result), seed: row.seed, options: row.options });
    }
  });
  return out;
}

async function obfuscated(rows, texts, bundles, cache) {
  let pending = rows.filter((row) => !cache.has(row.file));
  for (let number = 0; number < ATTEMPTS && pending.length > 0; number++) {
    const results = await obfuscateRows(pending.map((row) => attempt(row, number)), texts, bundles);
    for (const [file, result] of results) cache.set(file, result);
    pending = pending.filter((row) => cache.get(row.file).error === "the obfuscated output has no string array");
  }
}

async function generate(rows, cache, inputOf) {
  const fixtures = [];
  for (const row of rows) {
    const entry = cache.get(row.file);
    if (entry.code === undefined) continue;
    try {
      entry.built ??= await build(row, entry.code, inputOf(row));
      fixtures.push({ ...row, seed: entry.seed, options: entry.options, code: entry.built });
    } catch (error) {
      cache.set(row.file, { ...entry, error: error.message });
    }
  }
  return { fixtures, failed: rows.filter((row) => cache.get(row.file).error !== undefined) };
}

function clean() {
  mkdirSync(join(OUT, "expected"), { recursive: true });
  for (const name of readdirSync(OUT)) {
    if (name.endsWith(".js")) rmSync(join(OUT, name));
  }
}

function planned(allowed, bundles, excluded) {
  const versions = VERSIONS.map(versionOf);
  const packages = Object.fromEntries(VERSIONS.map((name) => [versionOf(name), name]));
  const rows = cover({ versions, programs: PROGRAMS, allowed: permitted(allowed, bundles), excluded }, COVER_SEED);
  return rows.map((row) => {
    const cell = cellName(row.version, row);
    return { ...row, cell, package: packages[row.version], file: `${cell}--${row.program}-1.js` };
  });
}

function manifestOf(fixtures) {
  const lines = [
    "# file\tcell\tsource\tseed\tobfuscator\tbehaviour\toptions\tsha256",
    `# generated by tools/obfuscated/versions.mjs with node ${process.version}, webpack@${versionOf("webpack")}, uglify-js@${versionOf("uglify-js")}, terser@${versionOf("terser-5")}; expected/<source>.stdout is node on the source, never on a fixture`,
  ];
  for (const f of fixtures) {
    const options = JSON.stringify({ obfuscator: f.options, container: f.container, minifier: f.minifier });
    lines.push([f.file, f.cell, relative(REPO, join(SOURCES, `${f.program}.js`)), f.seed, `javascript-obfuscator@${f.version}`, judgement(f), options, sha256(f.code)].join("\t"));
  }
  return `${lines.join("\n")}\n`;
}

async function plan() {
  mkdirSync(WORK, { recursive: true });
  const texts = Object.fromEntries(PROGRAMS.map((p) => [p, readFileSync(join(SOURCES, `${p}.js`), "utf8")]));
  const expected = Object.fromEntries(PROGRAMS.map((p) => [p, runNode(join(SOURCES, `${p}.js`))]));
  const bundles = {};
  for (const program of PROGRAMS) bundles[program] = await bundle(texts[program], `source-${program}`, false).catch(() => null);
  const allowed = await probe(texts, bundles);
  const excluded = new Set();
  const cache = new Map();
  for (;;) {
    const rows = planned(allowed, bundles, excluded);
    await obfuscated(rows, texts, bundles, cache);
    const { fixtures, failed } = await generate(rows, cache, (row) => (row.container === "wp4-bundle" ? bundles[row.program] : texts[row.program]));
    if (failed.length === 0) {
      await plainCheck(rows, texts, bundles, expected);
      return { fixtures, expected };
    }
    for (const row of failed) {
      process.stderr.write(`excluded ${row.version} ${row.program} ${row.extra}: ${cache.get(row.file).error}\n`);
      excluded.add(`${row.version}/${row.program}/${row.extra}`);
    }
  }
}

function write({ fixtures, expected }) {
  clean();
  for (const program of PROGRAMS) writeFileSync(join(OUT, "expected", `${program}.stdout`), expected[program]);
  for (const f of fixtures) writeFileSync(join(OUT, f.file), f.code);
  writeFileSync(join(OUT, "MANIFEST"), manifestOf(fixtures));
  const bytes = fixtures.reduce((sum, f) => sum + Buffer.byteLength(f.code), 0);
  process.stdout.write(`${new Set(fixtures.map((f) => f.cell)).size} cells, ${fixtures.length} fixtures, ${bytes} bytes\n`);
}

write(await plan());
