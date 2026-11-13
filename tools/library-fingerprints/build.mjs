import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "../..");
const require = createRequire(join(HERE, "package.json"));
const MODULES = join(HERE, "node_modules");

const BUNDLERS = ["rollup-cjs", "rollup-hoist", "esbuild", "webpack5"];
const MINIFIERS = ["none", "terser", "esbuild", "swc"];

const [, , id, workArg, ...rest] = process.argv;
if (!id || !workArg) {
  console.error("usage: node build.mjs <library-id> <work-dir> [--versions a,b] [--bundlers a,b] [--minifiers a,b] [--obfuscate none|latest|all|<versions>]");
  process.exit(2);
}
const options = {};
for (let i = 0; i < rest.length; i += 2) options[rest[i].replace(/^--/, "")] = rest[i + 1];
const WORK = resolve(workArg);
mkdirSync(WORK, { recursive: true });

const spec = (await import(pathToFileURL(join(HERE, "libraries", `${id}.mjs`)).href)).default;
const listed = (value, fallback) => (value ? value.split(",") : fallback);
const versions = spec.versions.filter((entry) => !options.versions || options.versions.split(",").includes(entry.version) || options.versions.split(",").includes(entry.dir));
const bundlers = listed(options.bundlers, spec.bundlers ?? BUNDLERS);
const minifiers = listed(options.minifiers, spec.minifiers ?? MINIFIERS);
const obfuscate = options.obfuscate ?? spec.obfuscate ?? "latest";
const version = (name) => JSON.parse(readFileSync(join(MODULES, name, "package.json"), "utf8")).version;

function modulesOf(entry) {
  const root = entry.root ?? spec.root;
  return root ? join(resolve(REPO, root), "node_modules") : MODULES;
}

function packageFiles(dir, modules) {
  const root = join(modules, dir);
  const out = {};
  const walk = (folder) => {
    for (const entry of require("node:fs").readdirSync(folder, { withFileTypes: true })) {
      const full = join(folder, entry.name);
      if (entry.isDirectory()) {
        if (entry.name !== "node_modules") walk(full);
      } else if (/\.(c|m)?js$/.test(entry.name)) {
        const rel = relative(root, full).split(sep).join("/");
        const module = spec.module(rel);
        if (module) out[rel] = module;
      }
    }
  };
  walk(root);
  return out;
}

function topLevelNames(code) {
  const names = [];
  const pattern = /^(?:var|let|const|function\*?|class|async function)\s+([A-Za-z_$][\w$]*)/gm;
  for (const match of code.matchAll(pattern)) names.push(match[1]);
  return names;
}

function relInPackage(path, dir) {
  const marker = `${sep}node_modules${sep}${dir}${sep}`;
  const at = path.indexOf(marker);
  return at < 0 ? null : path.slice(at + marker.length).split(sep).join("/");
}

async function bundleRollup(entry, dir, strict, modules) {
  const { rollup } = require("rollup");
  const commonjs = require("@rollup/plugin-commonjs");
  const { nodeResolve } = require("@rollup/plugin-node-resolve");
  const bundle = await rollup({
    input: entry,
    plugins: [nodeResolve({ preferBuiltins: false }), commonjs({ strictRequires: strict })],
    onwarn() {},
  });
  const { output } = await bundle.generate({ format: "cjs" });
  const names = {};
  const files = packageFiles(dir, modules);
  for (const [path, rendered] of Object.entries(output[0].modules)) {
    const rel = relInPackage(path.replace(/^\0/, "").replace(/\?.*$/, ""), dir);
    const module = rel && files[rel];
    if (!module) continue;
    for (const name of topLevelNames(rendered.code ?? "")) names[name] = module;
  }
  return { code: output[0].code, names };
}

async function bundleEsbuild(entry) {
  const { build } = require("esbuild");
  const result = await build({ entryPoints: [entry], bundle: true, write: false, format: "iife", platform: "browser", logLevel: "silent", absWorkingDir: dirname(entry) });
  return { code: result.outputFiles[0].text, names: {} };
}

async function bundleWebpack(entry, file) {
  const webpack = require("webpack");
  const out = dirname(file);
  const name = `${file.split(sep).pop()}.webpack.js`;
  await new Promise((done, fail) =>
    webpack(
      {
        mode: "production",
        context: dirname(entry),
        entry,
        target: "web",
        output: { path: out, filename: name },
        optimization: { minimize: false, moduleIds: "named", concatenateModules: false },
        performance: { hints: false },
      },
      (error, stats) => (error || stats.hasErrors() ? fail(error ?? new Error(stats.toString("errors-only"))) : done()),
    ),
  );
  return { code: readFileSync(join(out, name), "utf8"), names: {} };
}

async function minify(code, minifier) {
  if (minifier === "none") return code;
  if (minifier === "terser") return (await require("terser").minify(code, { mangle: false, compress: true })).code;
  if (minifier === "esbuild") return (await require("esbuild").transform(code, { minifySyntax: true, minifyWhitespace: true, minifyIdentifiers: false })).code;
  if (minifier === "swc") return (await require("@swc/core").minify(code, { mangle: false, compress: true })).code;
  throw new Error(`unknown minifier ${minifier}`);
}

function obfuscateCode(code) {
  const { obfuscate } = require("javascript-obfuscator");
  return obfuscate(code, {
    compact: true,
    renameGlobals: false,
    identifierNamesGenerator: "mangled",
    stringArray: true,
    stringArrayRotate: true,
    stringArrayShuffle: true,
    stringArrayThreshold: 1,
    stringArrayIndexShift: true,
    stringArrayWrappersCount: 1,
    seed: 1,
  }).getObfuscatedCode();
}

function wantsObfuscation(entry, index) {
  if (obfuscate === "none") return false;
  if (obfuscate === "all") return true;
  const wanted = obfuscate.split(",");
  return wanted.includes(entry.version) || (wanted.includes("latest") && index === versions.length - 1);
}

const manifest = [];
const packages = {};
const sha = (text) => createHash("sha256").update(text).digest("hex").slice(0, 16);
for (const [index, entry] of versions.entries()) {
  const modules = modulesOf(entry);
  const folder = join(modules, ".work", "library-fingerprints", id, entry.dir);
  mkdirSync(folder, { recursive: true });
  const entryFile = join(folder, "entry.js");
  writeFileSync(entryFile, spec.entry(entry.dir, entry.version));
  const files = packageFiles(entry.dir, modules);
  packages[entry.dir] = files;
  for (const bundler of bundlers) {
    let bundled;
    try {
      if (bundler === "rollup-cjs") bundled = await bundleRollup(entryFile, entry.dir, true, modules);
      else if (bundler === "rollup-hoist") bundled = await bundleRollup(entryFile, entry.dir, false, modules);
      else if (bundler === "esbuild") bundled = await bundleEsbuild(entryFile);
      else if (bundler === "webpack5") bundled = await bundleWebpack(entryFile, join(folder, bundler));
      else throw new Error(`unknown bundler ${bundler}`);
    } catch (error) {
      console.error(`${id} ${entry.version} ${bundler}: ${error.message.split("\n")[0]}`);
      continue;
    }
    for (const minifier of minifiers) {
      let code;
      try {
        code = await minify(bundled.code, minifier);
      } catch (error) {
        console.error(`${id} ${entry.version} ${bundler} ${minifier}: ${error.message.split("\n")[0]}`);
        continue;
      }
      const variants = [["none", code]];
      if (minifier === "terser" && wantsObfuscation(entry, index)) variants.push(["javascript-obfuscator", obfuscateCode(code)]);
      for (const [obfuscator, text] of variants) {
        const name = `${entry.dir}.${bundler}.${minifier}${obfuscator === "none" ? "" : ".obf"}.js`;
        const file = join(WORK, name);
        writeFileSync(file, text);
        manifest.push({ file: name, library: spec.label, version: entry.version, dir: entry.dir, bundler, minifier, obfuscator, names: bundled.names, sha: sha(text) });
      }
    }
  }
  for (const prebuilt of spec.prebuilt?.(entry.dir, entry.version, { require, MODULES: modules }) ?? []) {
    const name = `${entry.dir}.${prebuilt.name}.js`;
    writeFileSync(join(WORK, name), prebuilt.code);
    manifest.push({ file: name, library: spec.label, version: entry.version, dir: entry.dir, bundler: "prebuilt", minifier: prebuilt.minifier ?? "none", obfuscator: "none", names: prebuilt.names, sha: sha(prebuilt.code) });
  }
  console.error(`${id} ${entry.version}: ${manifest.length} builds so far`);
}
const tools = Object.fromEntries(["rollup", "@rollup/plugin-commonjs", "esbuild", "webpack", "terser", "@swc/core", "javascript-obfuscator"].map((name) => [name, version(name)]));
writeFileSync(join(WORK, "manifest.json"), JSON.stringify({ id, label: spec.label, versions: spec.versions.map((entry) => entry.version), tools, packages, builds: manifest }, null, 1));
