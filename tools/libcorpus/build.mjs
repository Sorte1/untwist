import { builtinModules, createRequire } from "node:module";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { apps } from "./apps.mjs";
import { owner } from "./roots.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT = process.env.UNTWIST_LIBCORPUS_OUT ?? join(process.env.HOME, ".cache/untwist-libcorpus");
const require = createRequire(join(HERE, "package.json"));
const BUILTINS = [...builtinModules, ...builtinModules.map((name) => `node:${name}`)];
const args = process.argv.slice(2);
const option = (flag, fallback) => (args.includes(flag) ? args[args.indexOf(flag) + 1] : fallback);
const only = new RegExp(option("--only", "."));
const [shard, shards] = option("--shard", "0/1").split("/").map(Number);
const bundlers = option("--bundlers", "webpack,rollup,esbuild,vite").split(",");

function writeTruth(dir, bundler, ids) {
  const rows = new Set();
  for (const id of ids) {
    const clean = id.replace(/^\0/, "").replace(/\?.*$/, "");
    const found = clean.includes("node_modules") ? owner(clean) : null;
    if (found) rows.add(`${found.lib}\t${found.version}\t${found.file}`);
  }
  writeFileSync(join(dir, `${bundler}.modules.tsv`), [...rows].sort().join("\n") + "\n");
  return rows.size;
}

const define = (env) => ({ "process.env.NODE_ENV": JSON.stringify(env) });

async function webpack(app, dir, entry) {
  const webpackLib = require("webpack");
  const config = {
    mode: "production", context: dirname(entry), entry, target: "web", devtool: false,
    output: { path: dir, filename: "webpack.js", iife: true },
    optimization: { minimize: false, splitChunks: false, runtimeChunk: false, nodeEnv: app.env },
    performance: { hints: false },
    resolve: { alias: { ...app.aliases }, fallback: Object.fromEntries(builtinModules.map((m) => [m, false])) },
    plugins: [new webpackLib.optimize.LimitChunkCountPlugin({ maxChunks: 1 })],
    module: { parser: { javascript: { importMeta: false } }, rules: [{ test: /\.m?js$/, resolve: { fullySpecified: false } }] },
  };
  const stats = await new Promise((resolve, reject) => webpackLib(config, (err, result) => (err ? reject(err) : resolve(result))));
  if (stats.hasErrors()) throw new Error(stats.toString({ all: false, errors: true }));
  const json = stats.toJson({ all: false, modules: true, nestedModules: true });
  const ids = [];
  const visit = (mods) => {
    for (const mod of mods ?? []) {
      if (mod.nameForCondition) ids.push(mod.nameForCondition);
      visit(mod.modules);
    }
  };
  visit(json.modules);
  return ids;
}

async function rollup(app, dir, entry) {
  const { rollup: run } = require("rollup");
  const load = async (name) => (await import(name)).default;
  const commonjs = await load("@rollup/plugin-commonjs");
  const nodeResolve = await load("@rollup/plugin-node-resolve");
  const alias = await load("@rollup/plugin-alias");
  const replace = await load("@rollup/plugin-replace");
  const json = await load("@rollup/plugin-json");
  const bundle = await run({
    input: entry, external: BUILTINS, onwarn: () => {},
    plugins: [
      alias({ entries: Object.entries(app.aliases).map(([find, to]) => ({ find, replacement: to })) }),
      replace({ preventAssignment: true, delimiters: ["(?<![\\w'\"`])", "(?![\\w'\"`])"], values: define(app.env) }),
      nodeResolve({ browser: true, preferBuiltins: false }),
      commonjs(), json(),
    ],
  });
  const { output } = await bundle.generate({ format: "iife", name: "app", inlineDynamicImports: true, globals: (id) => id });
  writeFileSync(join(dir, "rollup.js"), output[0].code);
  return Object.entries(output[0].modules).filter(([, info]) => info.renderedLength > 0).map(([id]) => id);
}

async function esbuild(app, dir, entry) {
  const { build } = require("esbuild");
  const result = await build({
    entryPoints: [entry], bundle: true, format: "iife", platform: "browser", outfile: join(dir, "esbuild.js"),
    logLevel: "silent", metafile: true, define: define(app.env), external: BUILTINS,
    alias: { ...app.aliases },
    absWorkingDir: dirname(entry),
  });
  const inputs = Object.values(result.metafile.outputs)[0].inputs;
  return Object.entries(inputs).filter(([, info]) => info.bytesInOutput > 0).map(([id]) => join(dirname(entry), id));
}

async function vite(app, dir, entry) {
  const { build } = await import(require.resolve("vite"));
  const result = await build({
    configFile: false, logLevel: "silent", root: dirname(entry), mode: app.env, define: define(app.env),
    resolve: { alias: Object.entries(app.aliases).map(([find, to]) => ({ find, replacement: to })) },
    build: { write: false, minify: false, modulePreload: false, rollupOptions: { input: entry, external: BUILTINS, output: { format: "iife" } } },
  });
  const outputs = (Array.isArray(result) ? result : [result]).flatMap((item) => item.output);
  const chunk = outputs.find((item) => item.type === "chunk");
  writeFileSync(join(dir, "vite.js"), chunk.code);
  return Object.entries(chunk.modules).filter(([, info]) => info.renderedLength > 0).map(([id]) => id);
}

const RUNNERS = { webpack, rollup, esbuild, vite };
const report = [];
for (const [index, app] of apps().entries()) {
  if (!only.test(app.id) || index % shards !== shard) continue;
  const dir = join(OUT, "bundles", app.id);
  mkdirSync(dir, { recursive: true });
  const appDir = join(HERE, app.root, "node_modules", ".apps", app.id);
  mkdirSync(appDir, { recursive: true });
  const entry = join(appDir, "index.js");
  writeFileSync(entry, app.source);
  writeFileSync(join(dir, "index.js"), app.source);
  for (const bundler of bundlers) {
    const started = Date.now();
    try {
      const ids = await RUNNERS[bundler](app, dir, entry);
      const count = writeTruth(dir, bundler, ids);
      report.push([app.id, bundler, "ok", count, Date.now() - started].join("\t"));
    } catch (error) {
      report.push([app.id, bundler, "error", String(error.message ?? error).split("\n")[0].slice(0, 200), Date.now() - started].join("\t"));
    }
    console.log(report.at(-1));
  }
}
writeFileSync(join(OUT, `build-report${option("--tag", "")}.tsv`), report.join("\n") + "\n");
