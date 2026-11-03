import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "../..");
const OUT = join(REPO, "fixtures/obfuscator-io");
const SOURCES = join(HERE, "sources");
const RUNNER = join(REPO, "tools/unpack-run.mjs");
const require = createRequire(join(HERE, "package.json"));
const PACKAGE = "javascript-obfuscator-5";
const Obfuscator = require(PACKAGE);
const acorn = require("acorn");

const PROGRAMS = [
  { name: "empty-log", namesReachOutside: false },
  { name: "greeting", namesReachOutside: false },
  { name: "functions", namesReachOutside: false },
  { name: "objects", namesReachOutside: true },
  { name: "classes", namesReachOutside: false },
  { name: "control", namesReachOutside: true },
  { name: "strings", namesReachOutside: false },
  { name: "mixed", namesReachOutside: true },
];
const EXTRA_SEEDS = { greeting: 2 };
const HOSTILE = ["debugProtection", "selfDefending", "domainLock"];
const NO_CODE_EFFECT = ["advertisement", "config", "exclude", "identifierNamesCache", "inputFileName", "log", "optionsPreset", "seed", "sourceMap", "sourceMapBaseUrl", "sourceMapFileName", "sourceMapMode", "sourceMapSourcesMode"];
const DICTIONARY = ["amber", "birch", "cedar", "dune", "ember", "fjord", "grove", "heath", "inlet", "jade", "kelp", "larch", "moss", "nook"];

const SINGLES = {
  "compact-false": { compact: false },
  "simplify-false": { simplify: false },
  "target-browser-no-eval": { target: "browser-no-eval" },
  "target-node": { target: "node" },
  "target-service-worker": { target: "service-worker" },
  "cff-075": { controlFlowFlattening: true },
  "cff-1": { controlFlowFlattening: true, controlFlowFlatteningThreshold: 1 },
  "dead-code-04": { deadCodeInjection: true },
  "dead-code-1": { deadCodeInjection: true, deadCodeInjectionThreshold: 1 },
  "debug-protection": { debugProtection: true },
  "debug-protection-interval": { debugProtection: true, debugProtectionInterval: 4000 },
  "disable-console": { disableConsoleOutput: true },
  "self-defending": { selfDefending: true },
  "domain-lock": { domainLock: ["example.com"] },
  "domain-lock-redirect": { domainLock: [".example.com", "example.org"], domainLockRedirectUrl: "https://example.com/locked" },
  "names-mangled": { identifierNamesGenerator: "mangled" },
  "names-mangled-shuffled": { identifierNamesGenerator: "mangled-shuffled" },
  "names-dictionary": { identifierNamesGenerator: "dictionary", identifiersDictionary: DICTIONARY },
  "names-prefix": { identifiersPrefix: "obf" },
  "rename-globals": { renameGlobals: true },
  "rename-properties-safe": { renameProperties: true },
  "rename-properties-unsafe": { renameProperties: true, renamePropertiesMode: "unsafe" },
  "numbers-to-expressions": { numbersToExpressions: true },
  "split-strings-10": { splitStrings: true },
  "split-strings-2": { splitStrings: true, splitStringsChunkLength: 2 },
  "object-keys": { transformObjectKeys: true },
  "unicode-escapes": { unicodeEscapeSequence: true },
  "string-array-off": { stringArray: false },
  "threshold-0": { stringArrayThreshold: 0 },
  "threshold-05": { stringArrayThreshold: 0.5 },
  "threshold-1": { stringArrayThreshold: 1 },
  "rotate-off": { stringArrayRotate: false },
  "shuffle-off": { stringArrayShuffle: false },
  "index-shift-off": { stringArrayIndexShift: false },
  "encoding-base64": { stringArrayEncoding: ["base64"] },
  "encoding-rc4": { stringArrayEncoding: ["rc4"] },
  "encoding-mixed": { stringArrayEncoding: ["none", "base64", "rc4"] },
  "indexes-string": { stringArrayIndexesType: ["hexadecimal-numeric-string"] },
  "indexes-mixed": { stringArrayIndexesType: ["hexadecimal-number", "hexadecimal-numeric-string"] },
  "calls-transform-05": { stringArrayCallsTransform: true },
  "calls-transform-1": { stringArrayCallsTransform: true, stringArrayCallsTransformThreshold: 1 },
  "wrappers-0": { stringArrayWrappersCount: 0 },
  "wrappers-2": { stringArrayWrappersCount: 2 },
  "wrappers-5": { stringArrayWrappersCount: 5 },
  "wrappers-unchained": { stringArrayWrappersCount: 2, stringArrayWrappersChainedCalls: false },
  "wrappers-function": { stringArrayWrappersType: "function" },
  "wrappers-function-params-5": { stringArrayWrappersCount: 2, stringArrayWrappersType: "function", stringArrayWrappersParametersMaxCount: 5 },
  "force-strings": { stringArrayThreshold: 0, forceTransformStrings: ["^l", "s$"] },
  "reserved-names": { reservedNames: ["^keep"] },
  "reserved-strings": { stringArrayThreshold: 1, reservedStrings: ["^keep", "^done"] },
  "ignore-imports": { stringArrayThreshold: 1, ignoreImports: true },
};

const PRESETS = {
  "preset-default": { optionsPreset: "default" },
  "preset-low": { optionsPreset: "low-obfuscation" },
  "preset-medium": { optionsPreset: "medium-obfuscation" },
  "preset-high": { optionsPreset: "high-obfuscation" },
};

const IMPACT = {
  cff: { controlFlowFlattening: true, controlFlowFlatteningThreshold: 1 },
  dead: { deadCodeInjection: true, deadCodeInjectionThreshold: 1 },
  rc4: { stringArrayEncoding: ["rc4"], stringArrayThreshold: 1 },
  wrap: { stringArrayWrappersCount: 3, stringArrayWrappersType: "function", stringArrayWrappersParametersMaxCount: 5 },
  calls: { stringArrayCallsTransform: true, stringArrayCallsTransformThreshold: 1 },
  split: { splitStrings: true, splitStringsChunkLength: 3 },
  num: { numbersToExpressions: true },
  keys: { transformObjectKeys: true },
  unicode: { unicodeEscapeSequence: true },
  self: { selfDefending: true },
  debug: { debugProtection: true, debugProtectionInterval: 4000 },
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

function uncovered() {
  const covered = new Set(Object.values(CELLS).flatMap((options) => Object.keys(options)));
  return Object.keys(Obfuscator.getOptionsByPreset("default")).filter((key) => !covered.has(key) && !NO_CODE_EFFECT.includes(key));
}

const sha256 = (text) => createHash("sha256").update(text).digest("hex");
const seedOf = (cell, program, index) => parseInt(sha256(`${cell}/${program}/${index}`).slice(0, 8), 16) % 1_000_000_000;

function runOriginal(file) {
  return execFileSync(process.execPath, ["--no-warnings", RUNNER, "--loader", "node", file], {
    cwd: dirname(file),
    encoding: "utf8",
    timeout: 30000,
  });
}

function judgement(options, program) {
  const preset = options.optionsPreset ? Obfuscator.getOptionsByPreset(options.optionsPreset) : {};
  const merged = { ...preset, ...options };
  if (HOSTILE.some((key) => (Array.isArray(merged[key]) ? merged[key].length > 0 : merged[key] === true))) return "residue-only:hostile";
  if (merged.renameProperties === true && program.namesReachOutside) return "residue-only:renamed-properties";
  return "node";
}

function obfuscate(code, options, seed, file) {
  const out = `${Obfuscator.obfuscate(code, { ...options, seed }).getObfuscatedCode()}\n`;
  if (out.includes(REPO) || out.includes(HERE)) throw new Error(`${file} embeds a local path`);
  acorn.parse(out, { ecmaVersion: "latest", sourceType: "script" });
  return out;
}

function clean() {
  mkdirSync(join(OUT, "expected"), { recursive: true });
  for (const name of readdirSync(OUT)) {
    if (name.endsWith(".js")) rmSync(join(OUT, name));
  }
}

function main() {
  const missing = uncovered();
  if (missing.length > 0) throw new Error(`no cell sets ${missing.join(", ")}`);
  clean();
  const version = JSON.parse(readFileSync(require.resolve(`${PACKAGE}/package.json`), "utf8")).version;
  const texts = {};
  for (const program of PROGRAMS) {
    const file = join(SOURCES, `${program.name}.js`);
    texts[program.name] = readFileSync(file, "utf8");
    writeFileSync(join(OUT, "expected", `${program.name}.stdout`), runOriginal(file));
  }
  const manifest = [
    "# file\tcell\tsource\tseed\tobfuscator\tbehaviour\toptions\tsha256",
    `# generated by tools/obfuscated/matrix.mjs with node ${process.version}; expected/<source>.stdout is node on the source, never on a fixture`,
  ];
  let bytes = 0;
  for (const [cell, options] of Object.entries(CELLS)) {
    for (const program of PROGRAMS) {
      const runs = 1 + (EXTRA_SEEDS[program.name] ?? 0);
      for (let index = 1; index <= runs; index++) {
        const name = `${cell}--${program.name}-${index}.js`;
        const seed = seedOf(cell, program.name, index);
        const code = obfuscate(texts[program.name], options, seed, name);
        writeFileSync(join(OUT, name), code);
        bytes += Buffer.byteLength(code);
        const source = relative(REPO, join(SOURCES, `${program.name}.js`));
        manifest.push([name, cell, source, seed, `javascript-obfuscator@${version}`, judgement(options, program), JSON.stringify(options), sha256(code)].join("\t"));
      }
    }
  }
  writeFileSync(join(OUT, "MANIFEST"), `${manifest.join("\n")}\n`);
  process.stdout.write(`${Object.keys(CELLS).length} cells, ${manifest.length - 2} fixtures, ${bytes} bytes\n`);
}

main();
