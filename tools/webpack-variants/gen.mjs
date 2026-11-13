import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "../..");
const OUT = join(REPO, "fixtures/webpack-variants");
const APP = join(HERE, "app");
const require = createRequire(join(HERE, "package.json"));

function version(alias) {
  return JSON.parse(readFileSync(require.resolve(`${alias}/package.json`), "utf8")).version;
}

function filesUnder(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir, { withFileTypes: true })
    .flatMap((d) => (d.isDirectory() ? filesUnder(join(dir, d.name)) : [join(dir, d.name)]))
    .sort();
}

function compile(webpack, config) {
  return new Promise((done, fail) => {
    webpack(config, (error, stats) => {
      if (error || stats.hasErrors()) return fail(error || new Error(stats.toString("errors-only")));
      const json = stats.toJson({ all: false, entrypoints: true });
      const assets = Object.values(json.entrypoints).flatMap((e) => e.assets.map((a) => a.name ?? a));
      done(assets.filter((name) => /\.m?js$/.test(name)));
    });
  });
}

const W4 = "webpack-4";
const W5 = ["webpack-5.24", "webpack-5.52", "webpack-5.75", "webpack-5.90", "webpack-5.99", "webpack"];
const LATEST = "webpack";

const short = (alias) => (alias === W4 ? "w4" : alias === LATEST ? `w${version(alias).split(".").slice(0, 2).join(".")}` : `w${alias.slice(8)}`);

function base(dir, extra = {}) {
  const { output = {}, optimization = {}, ...rest } = extra;
  return {
    context: APP,
    entry: { index: "./index.js" },
    target: "node",
    mode: "production",
    devtool: false,
    output: { path: dir, filename: "[name].js", chunkFilename: "[name].chunk.js", publicPath: "", ...output },
    optimization: { minimize: false, ...optimization },
    performance: { hints: false },
    ...rest,
  };
}

const OPTIONS = {
  node: { loader: "node", flags: "target=node", config: () => ({}) },
  "node-dev": { loader: "node", flags: "mode=development target=node", config: () => ({ mode: "development" }) },
  "node-min": { loader: "node", flags: "target=node minimize=true", config: () => ({ optimization: { minimize: true } }) },
  "async-node": { loader: "node", flags: "target=async-node", config: () => ({ target: "async-node" }) },
  "node-split": { loader: "node", flags: "target=node splitChunks=all minSize=0", config: () => ({ optimization: { splitChunks: { chunks: "all", minSize: 0 } } }) },
  "node-sourcemap": { loader: "node", flags: "target=node devtool=source-map", config: () => ({ devtool: "source-map" }) },
  web: { loader: "browser", flags: "target=web", config: () => ({ target: "web" }) },
  "web-dev": { loader: "browser", flags: "mode=development target=web", config: () => ({ mode: "development", target: "web" }) },
  "web-min": { loader: "browser", flags: "target=web minimize=true", config: () => ({ target: "web", optimization: { minimize: true } }) },
  "web-split": { loader: "browser", flags: "target=web splitChunks=all minSize=0", config: () => ({ target: "web", optimization: { splitChunks: { chunks: "all", minSize: 0 } } }) },
  "web-split-min": { loader: "browser", flags: "target=web splitChunks=all minSize=0 minimize=true", config: () => ({ target: "web", optimization: { minimize: true, splitChunks: { chunks: "all", minSize: 0 } } }) },
  "web-runtime-single": { loader: "browser", flags: "target=web runtimeChunk=single", config: () => ({ target: "web", optimization: { runtimeChunk: "single" } }) },
  "web-runtime-multiple": { loader: "browser", flags: "target=web runtimeChunk=multiple", config: () => ({ target: "web", optimization: { runtimeChunk: "multiple" } }) },
  "web-public-auto": { loader: "browser", flags: "target=web publicPath=auto", config: () => ({ target: "web", output: { publicPath: "auto" } }) },
  "web-sourcemap": { loader: "browser", flags: "target=web devtool=source-map", config: () => ({ target: "web", devtool: "source-map" }) },
  module: {
    loader: "node",
    flags: "target=node experiments.outputModule output.module chunkFormat=module chunkLoading=import",
    config: () => ({ experiments: { outputModule: true }, output: { module: true, filename: "[name].mjs", chunkFilename: "[name].chunk.mjs", chunkFormat: "module", chunkLoading: "import" } }),
  },
  umd: { loader: "node", flags: "target=node library.type=umd globalObject=this", config: () => ({ output: { library: { name: "variants", type: "umd" }, globalObject: "this" } }) },
  amd: { loader: "amd", flags: "target=node library.type=amd", config: () => ({ output: { library: { type: "amd" } } }) },
  system: { loader: "system", flags: "target=node library.type=system", config: () => ({ output: { library: { type: "system" } } }) },
  "web-es5": { loader: "browser", flags: "target=[web,es5]", config: () => ({ target: ["web", "es5"] }) },
  "web-noiife": { loader: "browser", flags: "target=web output.iife=false", config: () => ({ target: "web", output: { iife: false } }) },
  "web-federation": {
    loader: "browser",
    flags: "target=web ModuleFederationPlugin(name=variants exposes=./cjs shared=tinylib eager)",
    config: (webpack) => ({
      target: "web",
      plugins: [new webpack.container.ModuleFederationPlugin({ name: "variants", filename: "remoteEntry.js", exposes: { "./cjs": "./cjs.js" }, shared: { tinylib: { eager: true, singleton: true, requiredVersion: false } } })],
    }),
  },
  "web-module": {
    loader: "node",
    flags: "target=web experiments.outputModule output.module",
    config: () => ({ target: "web", experiments: { outputModule: true }, output: { module: true, filename: "[name].mjs", chunkFilename: "[name].chunk.mjs" } }),
  },
};

const W4_OPTIONS = ["node", "node-dev", "node-min", "async-node", "web", "web-dev", "web-min", "web-split", "web-runtime-single", "umd"];
const W5_EVERY = ["node", "web", "web-min", "web-split"];
const W5_LATEST = Object.keys(OPTIONS);
const MODULE_SINCE = "webpack-5.52";

function cells() {
  const list = W4_OPTIONS.map((option) => ({ alias: W4, option }));
  for (const alias of W5) {
    const options = alias === LATEST ? W5_LATEST : W5_EVERY.concat(alias === MODULE_SINCE || alias === "webpack-5.90" ? ["module", "node-split", "web-runtime-single"] : []);
    list.push(...options.map((option) => ({ alias, option })));
  }
  return list;
}

function w4Config(config) {
  const { output, ...rest } = config;
  const { module: _m, chunkFormat: _f, chunkLoading: _l, library, ...kept } = output ?? {};
  const legacy = library ? { library: library.name, libraryTarget: library.type } : {};
  return { ...rest, output: { ...kept, ...legacy } };
}

async function build({ alias, option }) {
  const name = `${short(alias)}-${option}`;
  const dir = join(OUT, name);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const webpack = require(alias);
  let config = base(dir, OPTIONS[option].config(webpack));
  if (alias === W4) config = w4Config(config);
  const entries = await compile(webpack, config);
  for (const map of filesUnder(dir).filter((f) => f.endsWith(".map"))) rmSync(map);
  const files = filesUnder(dir).map((f) => relative(dir, f));
  return { name, alias, option, entries, files };
}

function run(dir, loader, entries) {
  const out = spawnSync(process.execPath, ["--no-warnings", join(HERE, "run.mjs"), "--loader", loader, ...entries.map((e) => join(dir, e))], {
    cwd: dir,
    encoding: "utf8",
    timeout: 20000,
  });
  if (out.status !== 0) throw new Error(`${dir}: exit ${out.status}\n${out.stderr}`);
  return out.stdout;
}

async function main() {
  const only = process.argv[2];
  const expect = existsSync(join(OUT, "EXPECT")) ? readFileSync(join(OUT, "EXPECT"), "utf8") : null;
  if (!only) rmSync(OUT, { recursive: true, force: true });
  mkdirSync(OUT, { recursive: true });
  const rows = [];
  for (const cell of cells()) {
    if (only && !`${short(cell.alias)}-${cell.option}`.includes(only)) continue;
    const built = await build(cell);
    const dir = join(OUT, built.name);
    const stdout = run(dir, OPTIONS[cell.option].loader, built.entries);
    writeFileSync(join(OUT, `${built.name}.stdout`), stdout);
    const sha = createHash("sha256");
    for (const file of built.files) sha.update(readFileSync(join(dir, file)));
    rows.push([built.name, `webpack@${version(cell.alias)}`, OPTIONS[cell.option].loader, built.entries.join(","), built.files.join(","), OPTIONS[cell.option].flags, sha.digest("hex")].join("\t"));
    console.log(built.name, built.files.join(" "));
  }
  const kept = only && existsSync(join(OUT, "MANIFEST"))
    ? readFileSync(join(OUT, "MANIFEST"), "utf8").split("\n").filter((l) => l && !l.startsWith("#") && !rows.some((r) => r.split("\t")[0] === l.split("\t")[0]))
    : [];
  const all = [...kept, ...rows];
  if (expect !== null) writeFileSync(join(OUT, "EXPECT"), expect);
  writeFileSync(join(OUT, "MANIFEST"), "# cell\tbundler\tloader\tentries\tfiles\tflags\tsha256\n" + all.join("\n") + "\n");
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
