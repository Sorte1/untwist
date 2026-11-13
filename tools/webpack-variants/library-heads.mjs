import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "../..");
const OUT = join(REPO, "crates/untwist-unpack/tests/data/webpack_library_heads.txt");
const APP = join(HERE, "app");
const require = createRequire(join(HERE, "package.json"));
const PLACEHOLDER = "__webpack_bundle__";

const ALIASES = ["webpack-5.24", "webpack-5.52", "webpack-5.75", "webpack-5.90", "webpack-5.99", "webpack"];

const LIBRARIES = [
  ["umd", { name: "variants", type: "umd" }],
  ["umd2", { name: "variants", type: "umd2" }],
  ["umd-unnamed", { type: "umd" }],
  ["umd-nested-name", { name: ["outer", "inner"], type: "umd" }],
  ["umd-split-name", { name: { root: "Root", amd: "amd-name", commonjs: "cjs-name" }, type: "umd" }],
  ["amd", { type: "amd" }],
  ["amd-named", { name: "variants", type: "amd" }],
  ["amd-require", { type: "amd-require" }],
  ["system", { type: "system" }],
  ["system-named", { name: "variants", type: "system" }],
];

const TARGETS = [
  ["node", { target: "node" }],
  ["web", { target: "web" }],
  ["web-es5", { target: ["web", "es5"] }],
  ["global-this", { target: "node", globalObject: "this" }],
  ["global-globalThis", { target: "node", globalObject: "globalThis" }],
  ["global-window", { target: "web", globalObject: "window" }],
];

function compile(webpack, config) {
  return new Promise((done, fail) => {
    webpack(config, (error, stats) => {
      if (error || stats.hasErrors()) return fail(error || new Error(stats.toString("errors-only")));
      done();
    });
  });
}

function head(text) {
  const start = text.search(/\((?:\(\) => \{|function\(\) \{) \/\/ webpackBootstrap/);
  const returned = text.lastIndexOf("return __webpack_exports__;");
  const end = text.indexOf("})()", returned);
  if (start < 0 || returned < 0 || end < 0) throw new Error("no bootstrap found");
  return text.slice(0, start) + PLACEHOLDER + text.slice(end + 4);
}

async function build(alias, library, target) {
  const webpack = require(alias);
  const dir = mkdtempSync(join(tmpdir(), "untwist-heads-"));
  try {
    const { globalObject, ...rest } = target;
    await compile(webpack, {
      context: APP,
      entry: { index: "./index.js" },
      mode: "production",
      devtool: false,
      output: { path: dir, filename: "[name].js", chunkFilename: "[name].chunk.js", publicPath: "", library, ...(globalObject ? { globalObject } : {}) },
      optimization: { minimize: false },
      performance: { hints: false },
      ...rest,
    });
    return head(readFileSync(join(dir, "index.js"), "utf8"));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

async function main() {
  const seen = new Map();
  for (const alias of ALIASES) {
    for (const [name, library] of LIBRARIES) {
      for (const [targetName, target] of TARGETS) {
        const text = await build(alias, library, target);
        const label = `${name} ${targetName}`;
        if (!seen.has(text)) seen.set(text, label);
      }
    }
  }
  const sections = [...seen].map(([text, label]) => `//# ${label}\n${text.trim()}\n`);
  writeFileSync(OUT, sections.join(""));
  console.log(`${seen.size} heads`);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
