import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { Rng } from "./rng.mjs";
import { featuresOf, sourceOf } from "./grammar.mjs";
import { OBFUSCATORS, behaviourOf } from "./options.mjs";
import { apply, envelope, plan } from "./wiring.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "../..");
const PINS = join(REPO, "tools/obfuscated");
const require = createRequire(join(PINS, "package.json"));
const acorn = require("acorn");
const loaded = {};

function obfuscator(version) {
  loaded[version] ??= require(OBFUSCATORS[version]);
  return loaded[version];
}

export function sourceTypeOf(form) {
  return form === "esm" ? "module" : "script";
}

export function extensionOf(form) {
  return form === "esm" ? ".mjs" : ".js";
}

function parser(sourceType) {
  return (code) => acorn.parse(code, { ecmaVersion: "latest", sourceType, allowHashBang: true });
}

function compile(webpack, config) {
  return new Promise((done, fail) => {
    webpack(config, (error, stats) => {
      if (error || stats.hasErrors()) fail(error || new Error(stats.toString("errors-only")));
      else done();
    });
  });
}

function splitAt(spec) {
  return Math.ceil(spec.chunks.length / 2);
}

function writeOriginal(spec, dir) {
  const orig = join(dir, "orig");
  mkdirSync(orig, { recursive: true });
  const ext = extensionOf(spec.form);
  if (spec.form.startsWith("webpack")) {
    const at = splitAt(spec);
    writeFileSync(join(orig, "part.js"), sourceOf(spec.chunks.slice(at)));
    writeFileSync(join(orig, "index.js"), `require("./part.js");\n\n${sourceOf(spec.chunks.slice(0, at))}`);
  } else {
    writeFileSync(join(orig, `index${ext}`), envelope(sourceOf(spec.chunks), spec.envelope, sourceTypeOf(spec.form)));
  }
  return join(orig, `index${ext}`);
}

async function bundle(spec, dir, entry) {
  const work = join(dir, "bundle");
  const webpack = spec.form === "webpack4" ? require("webpack") : require("webpack5");
  const config = {
    context: dirname(entry),
    entry: "./index.js",
    target: "node",
    node: false,
    mode: spec.bundleMode,
    devtool: false,
    output: { path: work, filename: "bundle.js", hashFunction: "sha256" },
  };
  if (spec.form === "webpack4" && spec.bundleMode === "production") {
    const TerserPlugin = require("terser-webpack-plugin");
    config.optimization = { minimizer: [new TerserPlugin({ cache: false, parallel: false, extractComments: false })] };
  }
  await compile(webpack, config);
  const code = readFileSync(join(work, "bundle.js"), "utf8");
  rmSync(work, { recursive: true, force: true });
  return code;
}

function obfuscate(spec, code) {
  const out = obfuscator(spec.obfuscator).obfuscate(code, { ...spec.options, seed: spec.obfSeed }).getObfuscatedCode();
  if (out.includes(REPO) || out.includes(HERE)) throw new Error("the sample embeds a local path");
  return out;
}

function minify(spec, code) {
  if (!spec.terser) return { code, terser: false };
  const result = require("terser").minify(code, { module: spec.form === "esm", compress: { passes: 1 }, mangle: true });
  if (result.error) return { code, terser: false };
  return { code: result.code, terser: true };
}

function presetOptions(spec) {
  const preset = spec.options.optionsPreset;
  return preset ? obfuscator(spec.obfuscator).getOptionsByPreset(preset) : {};
}

export async function build(spec, dir) {
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const entry = writeOriginal(spec, dir);
  const out = { ...spec, features: featuresOf(spec.chunks), entry: "orig/" + entry.split("/orig/")[1] };
  const parse = parser(sourceTypeOf(spec.form));
  try {
    const input = spec.form.startsWith("webpack") ? await bundle(spec, dir, entry) : sourceOf(spec.chunks);
    const minified = minify(spec, obfuscate(spec, input));
    const steps = spec.wiring ?? plan(new Rng(`${spec.seed}/${spec.index}/plan`), parse(minified.code), out.features);
    const wired = apply(new Rng(`${spec.seed}/${spec.index}/apply`), steps, parse, minified.code);
    const sample = `${envelope(wired.code, spec.envelope, sourceTypeOf(spec.form))}\n`;
    parse(sample);
    const name = `sample${extensionOf(spec.form)}`;
    writeFileSync(join(dir, name), sample);
    Object.assign(out, { wiring: steps, applied: wired.applied, terserApplied: minified.terser, sample: name, behaviour: behaviourOf(spec.options, presetOptions(spec)), status: "ok" });
  } catch (error) {
    const message = String(error && error.message ? error.message : error);
    Object.assign(out, { status: "skip", skip: message.split("\n").find((line) => line.trim() !== "")?.trim().slice(0, 200) ?? "unknown" });
  }
  writeFileSync(join(dir, "spec.json"), `${JSON.stringify(out, null, 1)}\n`);
  return out;
}
