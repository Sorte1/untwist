import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { basename, dirname, extname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "../..");
const require = createRequire(join(HERE, "package.json"));
const acorn = require("acorn");

const SOURCES = [
  "tools/obfuscated/sources/empty-log.js",
  "tools/obfuscated/sources/test.js",
  "tools/obfuscated/sources/greeting.js",
  "tools/obfuscated/sources/functions.js",
  "tools/obfuscated/sources/objects.js",
  "tools/obfuscated/sources/classes.js",
  "tools/obfuscated/sources/control.js",
  "tools/obfuscated/sources/strings.js",
  "tools/obfuscated/sources/mixed.js",
  "tools/obfuscated/plain/script.cjs",
  "fixtures/commonjs/export-names.cjs",
  "fixtures/commonjs/removed-decoy-functions.cjs",
  "fixtures/libraries/tslib/rollup-cjs.js",
  "fixtures/libraries/regenerator-runtime/vite.none.js",
  "fixtures/libraries/swc-helpers/vite.none.js",
  "fixtures/libraries/babel-helpers/inline.webpack5.none.js",
  "tools/bla-sweep/sources/license.cjs",
];
const INTENSITIES = ["stealth", "balanced", "maximum", "bankrupt"];
const VARIANTS = ["default", "no-honey-map", "llm-directives", "self-seal"];
const ENCRYPT_SOURCES = ["tools/bla-sweep/sources/license.cjs"];
const LANES = ["default", "unsafe"];
const RANK = { perfect: 0, kept: 1, WRONG: 2 };
const PREDICATE = /\(Math\.imul\(([A-Za-z_$][\w$]*) [|^] /g;
const EXPECT_FILE = join(HERE, "EXPECT");
const DIGEST_FILE = join(HERE, "DIGESTS");

function parseArgs(argv) {
  const config = {
    untwist: process.env.UNTWIST_BIN ?? join(REPO, "target/release/untwist"),
    out: join(homedir(), ".cache/untwist-tmp/bla-sweep"),
    jobs: 6,
    timeout: 30,
    filter: null,
    write: false,
    keep: false,
  };
  for (let i = 0; i < argv.length; i++) {
    const flag = argv[i];
    const value = () => argv[++i];
    if (flag === "--untwist") config.untwist = resolve(value());
    else if (flag === "--out") config.out = resolve(value());
    else if (flag === "--jobs") config.jobs = Number(value());
    else if (flag === "--timeout") config.timeout = Number(value());
    else if (flag === "--filter") config.filter = new RegExp(value());
    else if (flag === "--write") config.write = true;
    else if (flag === "--keep") config.keep = true;
    else throw new Error(`unknown flag ${flag}`);
  }
  return config;
}

function sourceId(path) {
  const parts = path.split("/");
  const name = basename(path, extname(path));
  return parts[0] === "fixtures" && parts[1] === "libraries" ? `${parts[2]}-${name}` : name;
}

function plan() {
  const cases = [];
  for (const path of SOURCES) {
    const variants = ENCRYPT_SOURCES.includes(path) ? [...VARIANTS, "encrypt-strings"] : VARIANTS;
    for (const intensity of INTENSITIES) {
      for (const variant of variants) {
        const id = `${sourceId(path)}.${intensity}.${variant}`;
        cases.push({ id, source: path, name: basename(path), intensity, variant, seed: `untwist-bla27/${id}` });
      }
    }
  }
  return cases;
}

function exec(command, args, options = {}) {
  return new Promise((done) => {
    const child = spawn(command, args, { cwd: options.cwd, stdio: ["ignore", "pipe", "pipe"] });
    const out = [];
    const err = [];
    let size = 0;
    const take = (sink) => (chunk) => {
      size += chunk.length;
      if (size <= 64 * 1024 * 1024) sink.push(chunk);
    };
    child.stdout.on("data", take(out));
    child.stderr.on("data", take(err));
    child.on("close", (status, signal) => done({ status, signal, stdout: Buffer.concat(out).toString("utf8"), stderr: Buffer.concat(err).toString("utf8") }));
  });
}

function sandbox(binds, command) {
  const args = ["--unshare-all", "--die-with-parent", "--new-session", "--ro-bind", "/usr", "/usr", "--symlink", "usr/lib64", "/lib64", "--symlink", "usr/lib", "/lib", "--symlink", "usr/bin", "/bin", "--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp", "--clearenv", "--setenv", "PATH", "/usr/bin", "--setenv", "HOME", "/tmp", "--chdir", "/tmp"];
  for (const [mode, from, to] of binds) args.push(mode === "rw" ? "--bind" : "--ro-bind", from, to);
  return ["bwrap", [...args, ...command]];
}

async function generate(item, dir) {
  const protectedDir = join(dir, "protected");
  rmSync(protectedDir, { recursive: true, force: true });
  mkdirSync(protectedDir, { recursive: true });
  const spec = { input: `/in/${item.name}`, name: item.name, intensity: item.intensity, variant: item.variant, seed: item.seed };
  const [command, args] = sandbox(
    [["ro", HERE, "/gen"], ["ro", join(REPO, item.source), `/in/${item.name}`], ["rw", protectedDir, "/out"]],
    ["node", "/gen/gen.mjs", JSON.stringify(spec), "/out"],
  );
  const result = await exec(command, args);
  if (result.status !== 0) return { refused: (result.stderr.split("\n").find((line) => /Error/.test(line)) ?? result.stderr).slice(0, 300) };
  return { ...JSON.parse(readFileSync(join(protectedDir, "gen.json"), "utf8")), file: join(protectedDir, item.name) };
}

async function runNode(file, config) {
  const [command, args] = sandbox([["ro", dirname(file), "/case"]], ["timeout", "-k", "2", String(config.timeout), "node", "--max-old-space-size=256", `/case/${basename(file)}`]);
  const result = await exec(command, args);
  return { stdout: result.stdout, status: result.status, error: result.stderr.split("\n").find((line) => /^\w*Error\b/.test(line)) ?? "" };
}

async function untwist(file, lane, config) {
  const args = ["--report", "json", "--max-memory", "768MiB", "--source-type", "commonjs", ...(lane === "unsafe" ? ["--unsafe"] : []), file];
  const result = await exec(config.untwist, args);
  try {
    return JSON.parse(result.stdout);
  } catch {
    return { error: (result.stderr || `exit ${result.status}`).slice(0, 400) };
  }
}

function identifiers(code) {
  const names = new Set();
  try {
    for (const token of acorn.tokenizer(code, { ecmaVersion: "latest", allowHashBang: true, allowReturnOutsideFunction: true, sourceType: "script" })) {
      if (token.type === acorn.tokTypes.name) names.add(token.value);
    }
  } catch {
    return null;
  }
  return names;
}

function residue(code, original, protectedNames) {
  const predicates = [...code.matchAll(PREDICATE)].length;
  const names = identifiers(code);
  if (!names) return { predicates, added: -1, sample: [] };
  const added = [...names].filter((name) => protectedNames.has(name) && !original.has(name));
  return { predicates, added: added.length, sample: added.slice(0, 8) };
}

function same(a, b) {
  return a.stdout === b.stdout && a.status === b.status;
}

async function lane(item, laneName, dir, gen, expect, config) {
  const report = await untwist(gen.file, laneName, config);
  if (typeof report.code !== "string") return { verdict: "WRONG", reason: `untwist failed: ${report.error ?? "no code"}` };
  const outFile = join(dir, laneName, item.name);
  mkdirSync(dirname(outFile), { recursive: true });
  writeFileSync(outFile, report.code.endsWith("\n") ? report.code : `${report.code}\n`);
  const printed = await runNode(outFile, config);
  const left = residue(report.code, expect.originalNames, expect.protectedNames);
  const bytes = Buffer.byteLength(report.code);
  const base = { bytes, ...left, notes: (report.notes ?? []).length };
  if (!same(printed, expect.original)) return { ...base, verdict: "WRONG", reason: `prints ${JSON.stringify(printed.stdout.slice(0, 160))} exit ${printed.status} ${printed.error}` };
  return { ...base, verdict: left.predicates === 0 && left.added === 0 ? "perfect" : "kept" };
}

async function runCase(item, originals, config) {
  const dir = join(config.out, "cases", item.id);
  mkdirSync(dir, { recursive: true });
  const gen = await generate(item, dir);
  const head = { id: item.id, intensity: item.intensity, variant: item.variant };
  if (gen.refused) return { ...head, generator: "refused", reason: gen.refused };
  const original = originals.get(item.source);
  const protectedCode = readFileSync(gen.file, "utf8");
  const protectedRun = await runNode(gen.file, config);
  const row = { ...head, sha256: gen.protectedSha256, bytes: Buffer.byteLength(protectedCode), predicates: [...protectedCode.matchAll(PREDICATE)].length };
  if (!same(protectedRun, original.run)) return { ...row, generator: "broken", reason: `protected prints ${JSON.stringify(protectedRun.stdout.slice(0, 160))} exit ${protectedRun.status} ${protectedRun.error}` };
  const expect = { original: original.run, originalNames: original.names, protectedNames: identifiers(protectedCode) ?? new Set() };
  const lanes = {};
  for (const laneName of LANES) lanes[laneName] = await lane(item, laneName, dir, gen, expect, config);
  if (!config.keep && LANES.every((name) => lanes[name].verdict === "perfect")) rmSync(dir, { recursive: true, force: true });
  return { ...row, generator: "ok", lanes };
}

async function pool(items, jobs, work) {
  const results = new Array(items.length);
  let next = 0;
  const worker = async () => {
    while (next < items.length) {
      const index = next++;
      results[index] = await work(items[index]);
      if (process.stderr.isTTY) process.stderr.write(`\r${index + 1}/${items.length} ${items[index].id}`.padEnd(80));
    }
  };
  await Promise.all(Array.from({ length: Math.max(1, jobs) }, worker));
  return results;
}

function readTable(file) {
  if (!existsSync(file)) return new Map();
  return new Map(
    readFileSync(file, "utf8")
      .split("\n")
      .filter((line) => line.trim() && !line.startsWith("#"))
      .map((line) => {
        const [id, ...rest] = line.trim().split(/\s+/);
        return [id, rest];
      }),
  );
}

function outcome(row) {
  if (row.generator !== "ok") return [row.generator, row.generator];
  return LANES.map((name) => row.lanes[name].verdict);
}

function writeTables(rows) {
  writeFileSync(EXPECT_FILE, rows.map((row) => `${row.id} ${outcome(row).join(" ")}`).join("\n") + "\n");
  writeFileSync(DIGEST_FILE, rows.map((row) => `${row.id} ${row.sha256 ?? "-"}`).join("\n") + "\n");
}

function summarise(rows) {
  const table = {};
  for (const row of rows) {
    const key = `${row.intensity} ${row.variant}`;
    table[key] ??= { cases: 0, refused: 0, broken: 0, default: { perfect: 0, kept: 0, WRONG: 0 }, unsafe: { perfect: 0, kept: 0, WRONG: 0 } };
    const entry = table[key];
    entry.cases++;
    if (row.generator === "refused") entry.refused++;
    else if (row.generator === "broken") entry.broken++;
    else for (const name of LANES) entry[name][row.lanes[name].verdict]++;
  }
  return table;
}

function compare(rows) {
  const expected = readTable(EXPECT_FILE);
  const digests = readTable(DIGEST_FILE);
  const problems = [];
  const better = [];
  for (const row of rows) {
    if (row.lanes) for (const name of LANES) if (row.lanes[name].verdict === "WRONG") problems.push(`${row.id} ${name}: WRONG ${row.lanes[name].reason}`);
    const want = expected.get(row.id);
    const digest = digests.get(row.id);
    if (digest && row.sha256 && digest[0] !== row.sha256) problems.push(`${row.id}: generator output ${row.sha256} differs from DIGESTS ${digest[0]}`);
    if (!want) {
      problems.push(`${row.id}: not in EXPECT`);
      continue;
    }
    const got = outcome(row);
    got.forEach((verdict, index) => {
      if (verdict === want[index]) return;
      if (verdict in RANK && want[index] in RANK && RANK[verdict] < RANK[want[index]]) better.push(`${row.id} ${LANES[index]}: ${want[index]} -> ${verdict}`);
      else problems.push(`${row.id} ${LANES[index]}: ${want[index]} -> ${verdict}`);
    });
  }
  return { problems, better };
}

async function main() {
  const config = parseArgs(process.argv.slice(2));
  if (!existsSync(config.untwist)) throw new Error(`no untwist binary at ${config.untwist}`);
  if (!existsSync(join(HERE, "node_modules/bla-agents/package.json"))) throw new Error("run npm ci --ignore-scripts --prefix tools/bla-sweep first");
  mkdirSync(config.out, { recursive: true });
  const cases = plan().filter((item) => !config.filter || config.filter.test(item.id));
  const originals = new Map();
  for (const path of new Set(cases.map((item) => item.source))) {
    const file = join(REPO, path);
    originals.set(path, { run: await runNode(file, config), names: identifiers(readFileSync(file, "utf8")) ?? new Set() });
  }
  const rows = await pool(cases, config.jobs, (item) => runCase(item, originals, config));
  writeFileSync(join(config.out, "results.json"), JSON.stringify(rows, null, 1));
  const summary = summarise(rows);
  writeFileSync(join(config.out, "summary.json"), JSON.stringify(summary, null, 1));
  for (const [key, entry] of Object.entries(summary)) {
    const lanes = LANES.map((name) => `${name} ${entry[name].perfect}/${entry[name].kept}/${entry[name].WRONG}`).join("  ");
    console.log(`${key.padEnd(32)} ${String(entry.cases).padStart(3)} cases  refused ${entry.refused} broken ${entry.broken}  perfect/kept/WRONG ${lanes}`);
  }
  if (config.write) {
    if (config.filter) throw new Error("--write needs the whole plan, drop --filter");
    writeTables(rows);
    console.log(`wrote ${EXPECT_FILE} and ${DIGEST_FILE}`);
    return;
  }
  const { problems, better } = compare(rows);
  for (const line of better) console.log(`better: ${line}`);
  for (const line of problems) console.log(`FAIL: ${line}`);
  console.log(`${rows.length} cases, ${problems.length} problems, ${better.length} better than EXPECT`);
  process.exitCode = problems.length > 0 ? 1 : 0;
}

await main();
