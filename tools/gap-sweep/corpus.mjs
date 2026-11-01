import { spawn } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const CAP = ["--user", "--scope", "-q", "--slice=untwist.slice", "-p", "MemoryMax=12G", "-p", "MemorySwapMax=0"];
const UNTWIST_TIMEOUT_S = 900;
const REFERENCE_TIMEOUT_S = 5400;
const RESIDUE_TIMEOUT_S = 1800;
const NODE_HEAP_MB = 11000;
const NAME = /^(game|loader)-([0-9a-f]+)-(.+)\.js$/;
const LANES = [
  { lane: "default", flags: [] },
  { lane: "unsafe", flags: ["--unsafe"] },
];

function parseArgs(argv) {
  const options = { jobs: "2", mode: "untwist" };
  for (let i = 0; i < argv.length; i += 2) options[argv[i].replace(/^--/, "")] = argv[i + 1];
  if (!options.corpus || !options.out) throw new Error("usage: corpus.mjs --corpus <dir> --out <dir> [--untwist <bin>] [--reference <dist/index.js>] [--mode untwist|reference] [--mangle true|false] [--jobs N]");
  return { ...options, jobs: Number(options.jobs) };
}

function describe(dir, name) {
  const match = NAME.exec(name);
  if (!match) return undefined;
  const version = match[3];
  return { name, file: join(dir, name), kind: match[1], sha: match[2].slice(0, 12), version, minor: version.split(".").slice(0, 2).join("."), bytes: statSync(join(dir, name)).size };
}

function listCorpus(dir) {
  return readdirSync(dir).sort().map((name) => describe(dir, name)).filter(Boolean);
}

function readTiming(timeFile) {
  if (!existsSync(timeFile)) return undefined;
  const lines = readFileSync(timeFile, "utf8").trim().split("\n");
  const timing = lines.at(-1).split(" ");
  if (timing.length !== 2 || Number.isNaN(Number(timing[0]))) return undefined;
  const exited = /non-zero status (\d+)/.exec(lines.join("\n"));
  const signalled = /terminated by signal (\d+)/.exec(lines.join("\n"));
  const status = exited ? Number(exited[1]) : signalled ? 128 + Number(signalled[1]) : 0;
  return { status, wallS: Number(timing[0]), rssKb: Number(timing[1]) };
}

function run(command, args, timeoutS, stdoutPath) {
  return new Promise((resolve) => {
    const timeFile = `${stdoutPath}.time`;
    const child = spawn("systemd-run", [...CAP, "/usr/bin/time", "-f", "%e %M", "-o", timeFile, "timeout", String(timeoutS), command, ...args]);
    const out = [];
    const err = [];
    child.stdout.on("data", (chunk) => out.push(chunk));
    child.stderr.on("data", (chunk) => err.push(chunk));
    child.on("close", (status) => {
      writeFileSync(stdoutPath, Buffer.concat(out));
      const timing = readTiming(timeFile);
      resolve({ status: timing?.status ?? status, wallS: timing?.wallS ?? NaN, rssKb: timing?.rssKb ?? NaN, stderr: Buffer.concat(err).toString("utf8") });
    });
  });
}

function reuse(stdoutPath, stderrPath) {
  const timing = readTiming(`${stdoutPath}.time`);
  if (!timing || !existsSync(stdoutPath) || (stderrPath && !existsSync(stderrPath))) return undefined;
  return { ...timing, stderr: stderrPath ? readFileSync(stderrPath, "utf8") : "" };
}

function readRows(path) {
  if (!existsSync(path)) return [];
  return readFileSync(path, "utf8").split("\n").filter(Boolean).map((line) => JSON.parse(line));
}

function notesFrom(stderr) {
  return stderr.split("\n").filter((line) => /^(note|warning|error):/.test(line) || /panicked|thread '/.test(line));
}

async function measure(files, target) {
  const rowsPath = `${target}.residue.jsonl`;
  const done = new Map(readRows(rowsPath).filter((row) => !row.error).map((row) => [row.file, row]));
  const status = files.every((file) => done.has(file)) ? 0 : await measureFresh(files, target, rowsPath);
  const byFile = new Map(readRows(rowsPath).map((row) => [row.file, row]));
  return files.map((file) => byFile.get(file) ?? { file, error: `residue status ${status}` });
}

async function measureFresh(files, target, rowsPath) {
  const list = `${target}.list`;
  writeFileSync(list, files.join("\n"));
  const result = await run("node", [`--max-old-space-size=${NODE_HEAP_MB}`, join(HERE, "residue.mjs"), "--list", list], RESIDUE_TIMEOUT_S, rowsPath);
  return result.status;
}

async function untwistOne(options, entry) {
  const base = join(options.out, entry.name);
  const lanes = {};
  for (const lane of LANES) {
    const reportPath = `${base}.${lane.lane}.report.txt`;
    const output = `${base}.${lane.lane}.js`;
    const result = reuse(output, reportPath) ?? (await run(options.untwist, ["--single-file", "--rename", "simple", "--report", "text", ...lane.flags, entry.file], UNTWIST_TIMEOUT_S, output));
    writeFileSync(reportPath, result.stderr);
    lanes[lane.lane] = { status: result.status, wallS: result.wallS, rssKb: result.rssKb, timedOut: result.status === 124, notes: notesFrom(result.stderr) };
  }
  const [input, def, unsafe] = await measure([entry.file, `${base}.default.js`, `${base}.unsafe.js`], `${base}.untwist`);
  return { ...entry, input, untwist: lanes, residue: { default: def, unsafe } };
}

async function referenceOne(options, entry) {
  const base = join(options.out, entry.name);
  const list = `${base}.reference.tsv`;
  writeFileSync(list, `${entry.file}\t${base}.reference.js`);
  const statusPath = `${base}.reference.status.jsonl`;
  const result = reuse(statusPath) ?? (await run("node", [`--max-old-space-size=${NODE_HEAP_MB}`, join(HERE, "run-reference.mjs"), options.reference, list, options.mangle === "false" ? "no-mangle" : "mangle"], REFERENCE_TIMEOUT_S, statusPath));
  const ok = existsSync(`${base}.reference.js`) && statSync(`${base}.reference.js`).size > 0;
  const [residue] = ok ? await measure([`${base}.reference.js`], `${base}.reference`) : [{ error: `reference status ${result.status}` }];
  return { ...entry, reference: { status: result.status, wallS: result.wallS, rssKb: result.rssKb, timedOut: result.status === 124, ok }, residue: { reference: residue } };
}

function sample(entries) {
  const picked = [];
  for (const minor of [...new Set(entries.map((entry) => entry.minor))]) {
    const games = entries.filter((entry) => entry.minor === minor && entry.kind === "game").sort((a, b) => a.bytes - b.bytes);
    const loader = entries.filter((entry) => entry.minor === minor && entry.kind === "loader").sort((a, b) => a.bytes - b.bytes).at(-1);
    picked.push(...new Set([games[0], games.at(-1), loader].filter(Boolean)));
  }
  return picked;
}

async function pool(items, jobs, work) {
  const queue = [...items];
  const workers = Array.from({ length: jobs }, async () => {
    while (queue.length > 0) await work(queue.shift());
  });
  await Promise.all(workers);
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  mkdirSync(options.out, { recursive: true });
  const entries = listCorpus(options.corpus);
  const chosen = options.mode === "reference" ? sample(entries) : entries;
  const worker = options.mode === "reference" ? referenceOne : untwistOne;
  const ordered = [...chosen].sort((a, b) => b.bytes - a.bytes);
  await pool(ordered, options.jobs, async (entry) => {
    const target = join(options.out, `${entry.name}.${options.mode}.result.json`);
    if (existsSync(target)) return;
    const result = await worker(options, entry);
    writeFileSync(target, JSON.stringify(result));
    process.stdout.write(`${options.mode} ${entry.name.slice(0, 20)} ${entry.version}\n`);
  });
}

await main();
