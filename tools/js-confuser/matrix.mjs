import { createHash } from "node:crypto";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "../..");
const OUT = join(REPO, "fixtures/js-confuser");
const SHARED_SOURCES = join(REPO, "tools/obfuscated/sources");
const OWN_SOURCES = join(HERE, "sources");
const RUNNER = join(REPO, "tools/unpack-run.mjs");
const require = createRequire(join(HERE, "package.json"));
const PACKAGE = "js-confuser";
const JsConfuser = require(PACKAGE);
const acorn = require("acorn");
const MAX_BYTES = 400_000;
const SEED_TRIES = 6;

const PROGRAMS = ["empty-log", "greeting", "functions", "objects", "classes", "control", "strings", "mixed", "numbers"];
const OWN = new Set(["numbers"]);
const sourceOf = (program) => join(OWN.has(program) ? OWN_SOURCES : SHARED_SOURCES, `${program}.js`);
const HOSTILE = ["domainLock", "startDate", "endDate"];
const NEEDS_A_SOURCE_FUNCTION = ["countermeasures"];
const NO_CODE_EFFECT = [...NEEDS_A_SOURCE_FUNCTION, "preset", "target", "verbose", "debugComments", "sourceMap", "sourceFileName", "globalVariables", "customLocks"];

const SINGLES = {
  "compact-false": { compact: false },
  "hex-numbers": { hexadecimalNumbers: true },
  minify: { minify: true },
  "rename-variables": { renameVariables: true },
  "rename-globals": { renameVariables: true, renameGlobals: true },
  "rename-labels": { renameLabels: true },
  "names-hexadecimal": { renameVariables: true, identifierGenerator: "hexadecimal" },
  "names-mangled": { renameVariables: true, identifierGenerator: "mangled" },
  "names-number": { renameVariables: true, identifierGenerator: "number" },
  "names-zero-width": { renameVariables: true, identifierGenerator: "zeroWidth" },
  "names-chinese": { renameVariables: true, identifierGenerator: "chinese" },
  "cff-05": { controlFlowFlattening: 0.5 },
  "cff-1": { controlFlowFlattening: true },
  "global-concealing": { globalConcealing: true },
  "string-concealing": { stringConcealing: true },
  "string-concealing-05": { stringConcealing: 0.5 },
  "string-custom-encoding": { stringConcealing: true, customStringEncodings: [{ code: "function {fnName}(str){return str.split('').reverse().join('')}", encode: (str) => str.split("").reverse().join("") }] },
  "string-encoding": { stringEncoding: true },
  "string-splitting": { stringSplitting: true },
  "duplicate-literals": { duplicateLiteralsRemoval: true },
  dispatcher: { dispatcher: true },
  "dispatcher-no-length": { dispatcher: true, preserveFunctionLength: false },
  rgf: { rgf: true },
  "object-extraction": { objectExtraction: true },
  flatten: { flatten: true },
  "dead-code": { deadCode: true },
  calculator: { calculator: true },
  "moved-declarations": { movedDeclarations: true },
  "opaque-predicates": { opaquePredicates: true },
  "variable-masking": { variableMasking: true },
  "ast-scrambler": { astScrambler: true },
  pack: { pack: true },
  "lock-self-defending": { lock: { selfDefending: true } },
  "lock-anti-debug": { lock: { antiDebug: true } },
  "lock-tamper": { lock: { tamperProtection: true } },
  "lock-integrity": { lock: { integrity: true } },
  "lock-start-date": { lock: { startDate: Date.UTC(2020, 0, 1) } },
  "lock-end-date": { lock: { endDate: Date.UTC(2099, 0, 1) } },
  "lock-domain": { lock: { domainLock: ["example.com"] } },
};

const PRESETS = {
  "preset-low": { preset: "low" },
  "preset-medium": { preset: "medium" },
  "preset-high": { preset: "high" },
};

const IMPACT = {
  strings: { stringConcealing: true },
  rename: { renameVariables: true, renameGlobals: true },
  minify: { minify: true },
  scrambler: { astScrambler: true },
  dispatcher: { dispatcher: true },
  globals: { globalConcealing: true },
};

function pairs() {
  const names = Object.keys(IMPACT);
  const out = {};
  for (let i = 0; i < names.length; i++) {
    for (let j = i + 1; j < names.length; j++) {
      out[`pair-${names[i]}+${names[j]}`] = { ...IMPACT[names[i]], ...IMPACT[names[j]] };
    }
  }
  return out;
}

const CELLS = { ...SINGLES, ...PRESETS, ...pairs() };

function validOptions() {
  const text = readFileSync(require.resolve(`${PACKAGE}/dist/validateOptions.js`), "utf8");
  const pick = (name) => [...text.match(new RegExp(`var ${name} = new Set\\(\\[([^\\]]*)\\]`))[1].matchAll(/"([^"]+)"/g)].map((m) => m[1]);
  return { top: pick("validProperties"), lock: pick("validLockProperties") };
}

function uncovered() {
  const covered = new Set();
  for (const options of Object.values(CELLS)) {
    for (const [key, value] of Object.entries(options)) {
      covered.add(key);
      if (key === "lock") Object.keys(value).forEach((k) => covered.add(`lock.${k}`));
    }
  }
  const { top, lock } = validOptions();
  return [...top, ...lock.map((k) => `lock.${k}`)].filter((key) => !covered.has(key) && !NO_CODE_EFFECT.includes(key.replace(/^lock\./, "")));
}

const sha256 = (text) => createHash("sha256").update(text).digest("hex");
const seedOf = (cell, program, attempt) => parseInt(sha256(`${cell}/${program}/${attempt}`).slice(0, 8), 16) % 1_000_000_000;

function seeded(seed) {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function runNode(file, cwd) {
  const result = spawnSync("timeout", ["30", process.execPath, "--no-warnings", RUNNER, "--loader", "node", file], {
    cwd,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
  return { status: result.status, stdout: result.stdout };
}

function runOriginal(file) {
  return execFileSync(process.execPath, ["--no-warnings", RUNNER, "--loader", "node", file], {
    cwd: dirname(file),
    encoding: "utf8",
    timeout: 30000,
  });
}

async function obfuscate(code, options, seed, file) {
  const random = Math.random;
  Math.random = seeded(seed);
  try {
    const result = await JsConfuser.obfuscate(code, { target: "node", ...options });
    const out = `${typeof result === "string" ? result : result.code}\n`;
    if (out.includes(REPO) || out.includes(HERE)) throw new Error(`${file} embeds a local path`);
    acorn.parse(out, { ecmaVersion: "latest", sourceType: "script" });
    return out;
  } finally {
    Math.random = random;
  }
}

function merged(options) {
  const preset = options.preset ? JsConfuser.presets[options.preset] : {};
  return { ...preset, ...options, lock: { ...(preset.lock ?? {}), ...(options.lock ?? {}) } };
}

function judgement(options, ran, expected) {
  const lock = merged(options).lock;
  if (HOSTILE.some((key) => (Array.isArray(lock[key]) ? lock[key].length > 0 : lock[key] !== undefined))) return "residue-only:hostile";
  if (ran.status !== 0 || ran.stdout !== expected) return "residue-only:obfuscated-differs";
  return "node";
}

function clean() {
  mkdirSync(join(OUT, "expected"), { recursive: true });
  for (const name of readdirSync(OUT)) {
    if (name.endsWith(".js")) rmSync(join(OUT, name));
  }
}

async function smallest(cell, program, text, options) {
  let best;
  for (let attempt = 1; attempt <= SEED_TRIES; attempt++) {
    const seed = seedOf(cell, program, attempt);
    const code = await obfuscate(text, options, seed, `${cell}--${program}`);
    if (Buffer.byteLength(code) <= MAX_BYTES) return { seed, code };
    if (!best || code.length < best.code.length) best = { seed, code };
  }
  return best;
}

async function main() {
  const missing = uncovered();
  if (missing.length > 0) throw new Error(`no cell sets ${missing.join(", ")}`);
  clean();
  const version = JSON.parse(readFileSync(require.resolve(`${PACKAGE}/package.json`), "utf8")).version;
  const texts = {};
  const expected = {};
  for (const program of PROGRAMS) {
    const file = sourceOf(program);
    texts[program] = readFileSync(file, "utf8");
    expected[program] = runOriginal(file);
    writeFileSync(join(OUT, "expected", `${program}.stdout`), expected[program]);
  }
  const manifest = [
    "# file\tcell\tsource\tseed\tobfuscator\tbehaviour\toptions\tsha256",
    `# generated by tools/js-confuser/matrix.mjs with node ${process.version}; seed seeds Math.random for the obfuscator; expected/<source>.stdout is node on the source; behaviour residue-only:obfuscated-differs means node on the obfuscated file does not print what the source prints`,
  ];
  const work = mkdtempSync(join(tmpdir(), "untwist-jsc-"));
  let bytes = 0;
  try {
    for (const [cell, options] of Object.entries(CELLS)) {
      for (const program of PROGRAMS) {
        const name = `${cell}--${program}-1.js`;
        const { seed, code } = await smallest(cell, program, texts[program], options);
        writeFileSync(join(OUT, name), code);
        bytes += Buffer.byteLength(code);
        const ran = runNode(join(OUT, name), work);
        const source = relative(REPO, sourceOf(program));
        manifest.push([name, cell, source, seed, `js-confuser@${version}`, judgement(options, ran, expected[program]), JSON.stringify(options), sha256(code)].join("\t"));
      }
    }
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
  writeFileSync(join(OUT, "MANIFEST"), `${manifest.join("\n")}\n`);
  process.stdout.write(`${Object.keys(CELLS).length} cells, ${manifest.length - 2} fixtures, ${bytes} bytes\n`);
}

await main();
