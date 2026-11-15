import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "../..");
const OUT = join(REPO, "fixtures/obfuscator-relayer");
const SOURCES = join(HERE, "sources");
const RUNNER = join(REPO, "tools/unpack-run.mjs");
const require = createRequire(join(HERE, "package.json"));
const PACKAGE = "javascript-obfuscator-5";
const Obfuscator = require(PACKAGE);
const acorn = require("acorn");

const LAYERS = 4;
const MAX_BYTES = 200_000;
const PROGRAMS = ["greeting", "functions", "objects", "classes", "strings"];
const HOSTILE = ["debugProtection", "domainLock"];
const WRAPPED = { stringArrayRotate: true, stringArrayShuffle: true, stringArrayThreshold: 1, stringArrayWrappersCount: 2, stringArrayWrappersType: "function", stringArrayWrappersParametersMaxCount: 4 };

const SETS = {
  "preset-default": { optionsPreset: "default" },
  "preset-low": { optionsPreset: "low-obfuscation" },
  "preset-medium": { optionsPreset: "medium-obfuscation" },
  "preset-high": { optionsPreset: "high-obfuscation" },
  "medium-unguarded": { optionsPreset: "medium-obfuscation", selfDefending: false, disableConsoleOutput: false },
  "strings-base64": { ...WRAPPED, stringArrayEncoding: ["base64"] },
  "strings-rc4": { ...WRAPPED, stringArrayEncoding: ["rc4"] },
  cff: { controlFlowFlattening: true, controlFlowFlatteningThreshold: 1 },
  dead: { deadCodeInjection: true, deadCodeInjectionThreshold: 1 },
  self: { selfDefending: true },
  keys: { transformObjectKeys: true },
  split: { splitStrings: true, splitStringsChunkLength: 3 },
  "rename-globals": { renameGlobals: true },
};

const sha256 = (text) => createHash("sha256").update(text).digest("hex");
const seedOf = (set, program, layer) => parseInt(sha256(`relayer/${set}/${program}/${layer}`).slice(0, 8), 16) % 1_000_000_000;

function runNode(file) {
  try {
    return execFileSync(process.execPath, ["--no-warnings", RUNNER, "--loader", "node", file], {
      cwd: dirname(file),
      encoding: "utf8",
      timeout: 10000,
    });
  } catch (error) {
    return `failed: ${error.code ?? error.status}`;
  }
}

function hostile(options) {
  const preset = options.optionsPreset ? Obfuscator.getOptionsByPreset(options.optionsPreset) : {};
  const merged = { ...preset, ...options };
  return HOSTILE.some((key) => (Array.isArray(merged[key]) ? merged[key].length > 0 : merged[key] === true));
}

function silenced(options) {
  const preset = options.optionsPreset ? Obfuscator.getOptionsByPreset(options.optionsPreset) : {};
  return { ...preset, ...options }.disableConsoleOutput === true;
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

function chain(set, options, program, text, expected, version) {
  const rows = [];
  const capped = [];
  const broken = [];
  const behaviour = hostile(options) ? "residue-only:hostile" : "node";
  let code = text;
  for (let layer = 1; layer <= LAYERS; layer++) {
    const name = `${set}-n${layer}--${program}-1.js`;
    const seed = seedOf(set, program, layer);
    code = obfuscate(code, options, seed, name);
    if (Buffer.byteLength(code) > MAX_BYTES) {
      capped.push(`${name} (${Buffer.byteLength(code)} bytes)`);
      break;
    }
    const path = join(OUT, name);
    writeFileSync(path, code);
    if (behaviour === "node") {
      const printed = runNode(path);
      if (printed.startsWith("failed:") || (!silenced(options) && printed !== expected)) {
        rmSync(path);
        broken.push(`${name} (${printed.startsWith("failed:") ? printed : "prints differently"})`);
        break;
      }
    }
    const source = relative(REPO, join(SOURCES, `${program}.js`));
    rows.push([name, `${set}-n${layer}`, source, seed, `javascript-obfuscator@${version}`, behaviour, JSON.stringify({ ...options, layers: layer }), sha256(code)].join("\t"));
  }
  return { rows, capped, broken };
}

function main() {
  clean();
  const version = JSON.parse(readFileSync(require.resolve(`${PACKAGE}/package.json`), "utf8")).version;
  const rows = [];
  const capped = [];
  const broken = [];
  for (const program of PROGRAMS) {
    const file = join(SOURCES, `${program}.js`);
    const text = readFileSync(file, "utf8");
    const expected = runNode(file);
    writeFileSync(join(OUT, "expected", `${program}.stdout`), expected);
    for (const [set, options] of Object.entries(SETS)) {
      const made = chain(set, options, program, text, expected, version);
      rows.push(...made.rows);
      capped.push(...made.capped);
      broken.push(...made.broken);
    }
  }
  const manifest = [
    "# file\tcell\tsource\tseed\tobfuscator\tbehaviour\toptions\tsha256",
    `# generated by tools/obfuscated/relayer.mjs with node ${process.version}; layer n is javascript-obfuscator run on layer n-1 with the cell's options and its own seed; options.layers is n`,
    `# chains stop before a layer over ${MAX_BYTES} bytes: ${capped.length > 0 ? capped.join(", ") : "none"}`,
    `# chains stop before a layer the obfuscator broke, one that fails or prints differently in node: ${broken.length > 0 ? broken.join(", ") : "none"}`,
    ...rows,
  ];
  writeFileSync(join(OUT, "MANIFEST"), `${manifest.join("\n")}\n`);
  process.stdout.write(`${rows.length} fixtures, ${capped.length} capped, ${broken.length} broken\n`);
}

main();
