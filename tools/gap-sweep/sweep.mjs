import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { basename, dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const CAP = ["--user", "--scope", "-q", "--slice=untwist.slice", "-p", "MemoryMax=12G", "-p", "MemorySwapMax=0"];
const BIG_BYTES = 1_000_000;
const SMALL_TIMEOUT_S = 120;
const BIG_TIMEOUT_S = 3600;
const NODE_HEAP_MB = 11000;
const FIXTURE_DIRS = [
  "fixtures/reference/deobfuscate",
  "fixtures/reference/unpack",
  "fixtures/deobfuscate-gaps",
  "fixtures/eval",
  "fixtures/corpus-npm",
  "fixtures/behaviour",
];
const LANES = [
  { lane: "default", flags: [] },
  { lane: "unsafe", flags: ["--unsafe"] },
];

function parseArgs(argv) {
  const options = { repo: undefined, out: undefined, untwist: undefined, only: undefined };
  for (let i = 0; i < argv.length; i += 2) options[argv[i].replace(/^--/, "")] = argv[i + 1];
  if (!options.repo || !options.out || !options.untwist) {
    throw new Error("usage: sweep.mjs --repo <checkout> --out <dir> --untwist <binary> [--only big|small]");
  }
  return options;
}

function jsFiles(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((name) => /\.(m?js)$/.test(name))
    .sort()
    .map((name) => join(dir, name));
}

function collectInputs(repo) {
  const roots = [join(repo, "game.js"), join(repo, "game_old.js"), join(repo, ".worktrees/web/fixtures/generated/webpack-5mb.js")];
  const fixtures = FIXTURE_DIRS.flatMap((dir) => jsFiles(join(repo, dir)));
  return [...roots, ...fixtures]
    .filter((file) => existsSync(file))
    .map((file) => ({ file, id: relative(repo, file).replaceAll("/", "__"), bytes: statSync(file).size }));
}

function capped(command, args, timeoutS, extra = {}) {
  return spawnSync("systemd-run", [...CAP, "timeout", String(timeoutS), command, ...args], {
    maxBuffer: 1 << 30,
    encoding: "utf8",
    ...extra,
  });
}

function runUntwist(binary, input, out, lane) {
  const target = join(out, `${input.id}.${lane.lane}.js`);
  const timeoutS = input.bytes > BIG_BYTES ? BIG_TIMEOUT_S : SMALL_TIMEOUT_S;
  const started = Date.now();
  const result = capped(binary, ["--single-file", "--rename", "simple", "--report", "text", ...lane.flags, input.file], timeoutS);
  writeFileSync(target, result.stdout ?? "");
  writeFileSync(join(out, `${input.id}.${lane.lane}.report.txt`), result.stderr ?? "");
  return { status: result.status, ms: Date.now() - started };
}

function referenceEntry(repo) {
  const root = process.env.UNTWIST_REFERENCE_DIR ?? join(repo, "reference");
  const packages = join(root, "packages");
  const found = existsSync(packages) ? readdirSync(packages).map((name) => join(packages, name, "dist/index.js")).find((path) => existsSync(path)) : undefined;
  if (!found) throw new Error(`the reference deobfuscator is not built under ${root}; set UNTWIST_REFERENCE_DIR`);
  return found;
}

function runReference(repo, inputs, out, timeoutS, tag) {
  const list = join(out, `reference-list.${tag}.tsv`);
  writeFileSync(list, inputs.map((input) => `${input.file}\t${join(out, `${input.id}.reference.js`)}`).join("\n"));
  const entry = referenceEntry(repo);
  const result = capped("node", [`--max-old-space-size=${NODE_HEAP_MB}`, join(HERE, "run-reference.mjs"), entry, list], timeoutS);
  const lines = (result.stdout ?? "").split("\n").filter(Boolean).map((line) => JSON.parse(line));
  const byInput = new Map(lines.map((line) => [line.input, line]));
  return inputs.map((input) => byInput.get(input.file) ?? { input: input.file, ok: false, error: `runner status ${result.status}: ${(result.stderr ?? "").slice(-300)}` });
}

function measureAll(files, out, tag) {
  const list = join(out, `residue-list.${tag}.txt`);
  writeFileSync(list, files.join("\n"));
  const result = capped("node", [`--max-old-space-size=${NODE_HEAP_MB}`, join(HERE, "residue.mjs"), "--list", list], BIG_TIMEOUT_S);
  const rows = (result.stdout ?? "").split("\n").filter(Boolean).map((line) => JSON.parse(line));
  return new Map(rows.map((row) => [row.file, row]));
}

const EXEC_TIMEOUT_S = 10;
const EXEC_DIRS = ["fixtures__behaviour__", "fixtures__reference__deobfuscate__", "fixtures__eval__", "fixtures__corpus-npm__", "fixtures__deobfuscate-gaps__"];

function execute(file, cwd) {
  const result = capped("node", [file], EXEC_TIMEOUT_S, { cwd });
  return `${result.status}\n${result.stdout ?? ""}`;
}

function execCheck(input, out) {
  if (!input.file.endsWith(".js") || !EXEC_DIRS.some((prefix) => input.id.startsWith(prefix))) return undefined;
  const cwd = dirname(input.file);
  const expected = execute(input.file, cwd);
  return Object.fromEntries(
    ["default", "unsafe", "reference"].map((lane) => {
      const target = join(out, `${input.id}.${lane}.js`);
      const exec = join(out, `${input.id}.${lane}.exec.js`);
      writeFileSync(exec, readFileSync(target, "utf8"));
      return [lane, execute(exec, cwd) === expected];
    }),
  );
}

function notesOf(out, id, lane) {
  const path = join(out, `${id}.${lane}.report.txt`);
  if (!existsSync(path)) return [];
  return readFileSync(path, "utf8")
    .split("\n")
    .filter((line) => line.startsWith("note:") || line.startsWith("warning:") || line.startsWith("error:"));
}

function sweep(options, inputs, tag) {
  const out = options.out;
  const runs = inputs.map((input) => ({
    input,
    untwist: Object.fromEntries(LANES.map((lane) => [lane.lane, runUntwist(options.untwist, input, out, lane)])),
  }));
  const timeoutS = inputs.some((input) => input.bytes > BIG_BYTES) ? BIG_TIMEOUT_S : SMALL_TIMEOUT_S * inputs.length;
  const reference = runReference(options.repo, inputs, out, timeoutS, tag);
  const files = inputs.flatMap((input) => [input.file, ...["default", "unsafe", "reference"].map((lane) => join(out, `${input.id}.${lane}.js`))]);
  const residue = measureAll(files, out, tag);
  return runs.map((run, index) => ({
    id: run.input.id,
    file: run.input.file,
    bytes: run.input.bytes,
    untwist: run.untwist,
    reference: reference[index],
    residue: {
      input: residue.get(run.input.file),
      ...Object.fromEntries(["default", "unsafe", "reference"].map((lane) => [lane, residue.get(join(out, `${run.input.id}.${lane}.js`))])),
    },
    sameOutput: execCheck(run.input, out),
    notes: { default: notesOf(out, run.input.id, "default"), unsafe: notesOf(out, run.input.id, "unsafe") },
  }));
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  mkdirSync(options.out, { recursive: true });
  const inputs = collectInputs(options.repo);
  const small = inputs.filter((input) => input.bytes <= BIG_BYTES);
  const big = inputs.filter((input) => input.bytes > BIG_BYTES);
  const groups = options.only === "big" ? [] : [["small", small]];
  const bigGroups = options.only === "small" ? [] : big.map((input) => [basename(input.file), [input]]);
  for (const [tag, group] of [...groups, ...bigGroups]) {
    const rows = sweep(options, group, tag);
    writeFileSync(join(options.out, `results.${tag}.json`), JSON.stringify(rows, null, 1));
    process.stdout.write(`${tag}: ${rows.length} inputs\n`);
  }
}

main();
