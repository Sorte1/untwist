import { createHash } from "node:crypto";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "../..");
const OUT = join(REPO, "fixtures/encoders");
const SOURCES = join(HERE, "sources");
const SHARED_SOURCES = join(REPO, "tools/obfuscated/sources");
const RUNNER = join(REPO, "tools/unpack-run.mjs");
const require = createRequire(join(HERE, "package.json"));
const { JSFuck } = require("jsfuck");
const JScrewIt = require("jscrewit");
const aaencode = require("mangle-js-webpack-plugin/util/aaencode.js");
const jjencode = require("mangle-js-webpack-plugin/util/jjencode.js");

const PROGRAMS = ["tiny", "words", "empty-log", "greeting", "strict", "lexical", "closures"];
const SHARED = new Set(["empty-log", "greeting"]);
const sourceOf = (program) => join(SHARED.has(program) ? SHARED_SOURCES : SOURCES, `${program}.js`);
const version = (name) => require(`${name}/package.json`).version;
const XOR_KEY = 23;

const ENCODERS = {
  jsfuck: { tool: () => `jsfuck@${version("jsfuck")}`, options: "wrapWithEval", encode: (text) => JSFuck.encode(text, true) },
  jscrewit: { tool: () => `jscrewit@${version("jscrewit")}`, options: "{}", encode: (text) => JScrewIt.encode(text) },
  "jscrewit-node": {
    tool: () => `jscrewit@${version("jscrewit")}`,
    options: '{"features":"NODE_22"}',
    encode: (text) => JScrewIt.encode(text, { features: "NODE_22" }),
  },
  "jscrewit-eval": {
    tool: () => `jscrewit@${version("jscrewit")}`,
    options: '{"runAs":"eval"}',
    encode: (text) => JScrewIt.encode(text, { runAs: "eval" }),
  },
  aaencode: {
    tool: () => `mangle-js-webpack-plugin@${version("mangle-js-webpack-plugin")} util/aaencode`,
    options: "{}",
    encode: (text) => aaencode(text).getObfuscatedCode(),
  },
  "jjencode-dollar": {
    tool: () => `mangle-js-webpack-plugin@${version("mangle-js-webpack-plugin")} util/jjencode`,
    options: '{"prefix":"$"}',
    encode: (text) => jjencode(text, { prefix: "$" }).getObfuscatedCode(),
  },
  "jjencode-word": {
    tool: () => `mangle-js-webpack-plugin@${version("mangle-js-webpack-plugin")} util/jjencode`,
    options: '{"prefix":"thinkzhan"}',
    encode: (text) => jjencode(text, { prefix: "thinkzhan" }).getObfuscatedCode(),
  },
  "loader-xor": {
    tool: () => "matrix.mjs",
    options: "eval of a char-code array mapped through a callback",
    encode: (text) =>
      `var key = ${XOR_KEY};\neval([${[...text].map((c) => c.charCodeAt(0) ^ XOR_KEY).join(",")}].map(function (c) { return String.fromCharCode(c ^ key); }).join(""));`,
  },
  "loader-reverse": {
    tool: () => "matrix.mjs",
    options: "Function of reversed chunks joined through a callback",
    encode: (text) => {
      const chunks = [];
      for (let at = 0; at < text.length; at += 24) chunks.push([...text.slice(at, at + 24)].reverse().join(""));
      return `var parts = ${JSON.stringify(chunks)};\nFunction(parts.map(function (part) { return part.split("").reverse().join(""); }).join(""))();`;
    },
  },
};

const sha256 = (text) => createHash("sha256").update(text).digest("hex");

function runNode(file) {
  const result = spawnSync("timeout", ["30", process.execPath, "--no-warnings", RUNNER, "--loader", "node", file], {
    cwd: dirname(file),
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

function clean() {
  mkdirSync(join(OUT, "expected"), { recursive: true });
  for (const name of readdirSync(OUT)) {
    if (name.endsWith(".js")) rmSync(join(OUT, name));
  }
}

function main() {
  clean();
  const expected = {};
  for (const program of PROGRAMS) {
    expected[program] = runOriginal(sourceOf(program));
    writeFileSync(join(OUT, "expected", `${program}.stdout`), expected[program]);
  }
  const rows = [];
  for (const [cell, encoder] of Object.entries(ENCODERS)) {
    for (const program of PROGRAMS) {
      const text = readFileSync(sourceOf(program), "utf8");
      const code = `${encoder.encode(text)}\n`;
      const file = `${cell}--${program}-1.js`;
      writeFileSync(join(OUT, file), code);
      const ran = runNode(join(OUT, file));
      const behaviour = ran.status === 0 && ran.stdout === expected[program] ? "node" : "residue-only:obfuscated-differs";
      const source = relative(REPO, sourceOf(program));
      rows.push([file, cell, source, 0, encoder.tool(), behaviour, encoder.options, sha256(code)].join("\t"));
    }
  }
  const header = [
    "# file\tcell\tsource\tseed\tobfuscator\tbehaviour\toptions\tsha256",
    `# generated by tools/encoders/matrix.mjs with node ${process.version}; the encoders are deterministic, so seed is 0; expected/<source>.stdout is node on the source; behaviour residue-only:obfuscated-differs means node on the encoded file does not print what the source prints`,
  ];
  writeFileSync(join(OUT, "MANIFEST"), `${[...header, ...rows].join("\n")}\n`);
  console.log(`${rows.length} fixtures in ${relative(REPO, OUT)}`);
}

main();
