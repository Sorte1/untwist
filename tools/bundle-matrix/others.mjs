import { execFileSync } from "node:child_process";
import { copyFileSync, createWriteStream, mkdirSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, relative } from "node:path";

import { HERE, WORK, bun, fresh, jsFiles, require, tool, version } from "./lib.mjs";

const ESBUILD = ["esbuild-0.14", "esbuild-0.17", "esbuild-0.19", "esbuild-0.21", "esbuild-0.24", "esbuild-0.25", "esbuild"];
const ESBUILD_MORE = ["esbuild-0.15", "esbuild-0.16", "esbuild-0.18", "esbuild-0.20", "esbuild-0.22", "esbuild-0.23", "esbuild-0.26", "esbuild-0.27"];
const BUNS = ["bun-1.1", "bun-1.2", "bun-1.3"];
const PACKAGES_BUNDLE = ["esbuild-0.22"];
const PARCELS = ["parcel-2.9", "parcel-2.12"];
const CREATE_REQUIRE = 'import { createRequire } from "module"; const require = createRequire(import.meta.url);';

function esbuildCell(alias, format, { minify = false, split = false, keepNames = false, target, platform = "node" } = {}) {
  const short = alias === "esbuild" ? version("esbuild").split(".").slice(0, 2).join(".") : alias.slice("esbuild-".length);
  const parts = [`esbuild${short}`, format, split && "split", minify && "min", keepNames && "keepnames", target, platform === "browser" && "browser"];
  return {
    name: parts.filter(Boolean).join("-"),
    family: "esbuild",
    tool: () => tool("esbuild", alias),
    loader: "node",
    flags: [`--bundle --platform=${platform} --format=${format}`, split && "--splitting", minify && "--minify", keepNames && "--keep-names", target && `--target=${target}`, PACKAGES_BUNDLE.includes(alias) && "--packages=bundle"]
      .filter(Boolean).join(" "),
    build: async (ctx) => {
      const { build } = require(alias);
      const result = await build({
        entryPoints: { index: join(ctx.srcDir, "index.js") }, bundle: true, format, minify, splitting: split, keepNames,
        platform, target, outdir: ctx.dir, chunkNames: "[name]-[hash].chunk", logLevel: "silent", metafile: true,
        ...(PACKAGES_BUNDLE.includes(alias) ? { packages: "bundle" } : {}),
        banner: format === "esm" ? { js: CREATE_REQUIRE } : undefined,
      });
      return Object.keys(result.metafile.inputs).length;
    },
  };
}

export const esbuild = [
  ...ESBUILD.flatMap((alias) => [
    esbuildCell(alias, "iife"),
    esbuildCell(alias, "cjs"),
    esbuildCell(alias, "esm", { split: true }),
    esbuildCell(alias, "iife", { minify: true }),
  ]),
  esbuildCell("esbuild", "iife", { keepNames: true }),
  esbuildCell("esbuild", "iife", { minify: true, keepNames: true }),
  esbuildCell("esbuild", "iife", { target: "es2015" }),
  esbuildCell("esbuild", "iife", { target: "esnext" }),
  esbuildCell("esbuild", "iife", { platform: "browser" }),
  ...ESBUILD_MORE.flatMap((alias) => [
    esbuildCell(alias, "iife"),
    esbuildCell(alias, "cjs"),
    esbuildCell(alias, "esm", { split: true }),
    esbuildCell(alias, "iife", { minify: true }),
  ]),
  ...["esbuild-0.14", "esbuild-0.21", "esbuild"].flatMap((alias) => [
    esbuildCell(alias, "esm"),
    esbuildCell(alias, "esm", { split: true, minify: true }),
  ]),
  esbuildCell("esbuild", "esm", { minify: true }),
  esbuildCell("esbuild", "cjs", { minify: true }),
  esbuildCell("esbuild", "cjs", { keepNames: true }),
  esbuildCell("esbuild", "esm", { split: true, keepNames: true }),
  esbuildCell("esbuild", "iife", { minify: true, platform: "browser" }),
  esbuildCell("esbuild", "iife", { target: "es2017" }),
  esbuildCell("esbuild", "iife", { minify: true, target: "es2015" }),
];

function bunBinary(alias) {
  return alias ? join(dirname(require.resolve(`${alias}/package.json`)), "bin", "bun") : "bun";
}

function bunVersion(alias) {
  return alias ? version(`${alias}/package.json`) : bun();
}

function bunCell(format, extra = [], alias = null) {
  const label = extra.map((a) => a.replace(/^--/, "").replace(/=.*/, "")).join("-");
  const prefix = alias ? `bun${alias.slice("bun-".length)}` : "bun";
  return {
    name: [prefix, format, label].filter(Boolean).join("-"),
    family: "esbuild",
    tool: () => `bun@${bunVersion(alias)}`,
    loader: "node",
    flags: `build --target=node --format=${format} ${extra.join(" ")}`.trim(),
    available: () => bunVersion(alias) !== null,
    build: async (ctx) => {
      const args = ["build", join(ctx.srcDir, "index.js"), "--target=node", `--format=${format}`, "--outdir", ctx.dir, "--entry-naming=[name].js", "--chunk-naming=[name]-[hash].chunk.js", ...extra];
      execFileSync(bunBinary(alias), args, { cwd: ctx.srcDir, stdio: "ignore" });
      return null;
    },
  };
}

export const buns = [
  bunCell("cjs"), bunCell("esm"), bunCell("esm", ["--splitting"]), bunCell("iife", ["--minify"]),
  bunCell("iife"), bunCell("cjs", ["--minify"]), bunCell("esm", ["--minify"]), bunCell("esm", ["--splitting", "--minify"]),
  bunCell("iife", ["--minify-syntax"]), bunCell("iife", ["--minify-whitespace"]), bunCell("iife", ["--minify-identifiers"]),
  ...BUNS.flatMap((alias) => [
    bunCell("cjs", [], alias), bunCell("esm", [], alias), bunCell("esm", ["--splitting"], alias),
    bunCell("iife", [], alias), bunCell("iife", ["--minify"], alias),
  ]),
  bunCell("esm", [], "bun-1.0"), bunCell("esm", ["--splitting"], "bun-1.0"), bunCell("esm", ["--minify"], "bun-1.0"),
];

function parcelRequire(dir) {
  return dir ? createRequire(join(HERE, dir, "package.json")) : require;
}

function parcelCell(scopeHoist, optimize, dir = null) {
  const from = parcelRequire(dir);
  const prefix = dir ? dir.replace("-", "") : "parcel";
  return {
    name: `${prefix}-${scopeHoist ? "hoist" : "nohoist"}${optimize ? "-min" : ""}`,
    family: "parcel",
    tool: () => (dir ? `parcel@${JSON.parse(readFileSync(from.resolve("@parcel/core/package.json"), "utf8")).version}` : tool("parcel")),
    loader: "node",
    flags: `context=node outputFormat=commonjs optimize=${optimize} scopeHoist=${scopeHoist}`,
    build: async (ctx) => {
      const { Parcel } = from("@parcel/core");
      const distDir = fresh(join(WORK, "parcel-out"));
      const bundler = new Parcel({
        entries: join(ctx.srcDir, "index.js"), defaultConfig: from.resolve("@parcel/config-default"), shouldDisableCache: true,
        cacheDir: join(WORK, "parcel-cache"), mode: "production", shouldContentHash: false, logLevel: "error",
        defaultTargetOptions: { shouldOptimize: optimize, shouldScopeHoist: scopeHoist, sourceMaps: false },
        targets: { app: { distDir, context: "node", outputFormat: "commonjs", engines: { node: ">= 22" }, includeNodeModules: true, isLibrary: false, optimize, scopeHoist, sourceMap: false } },
      });
      const result = await bundler.run();
      const assets = new Set();
      for (const b of result.bundleGraph.getBundles()) b.traverseAssets((a) => { if (a.type === "js" || a.type === "json") assets.add(a.filePath); });
      for (const file of jsFiles(distDir)) {
        const target = join(ctx.dir, relative(distDir, file));
        mkdirSync(join(target, ".."), { recursive: true });
        copyFileSync(file, target);
      }
      return assets.size;
    },
  };
}

export const parcel = [null, ...PARCELS].flatMap((dir) => [
  parcelCell(true, false, dir), parcelCell(false, false, dir), parcelCell(true, true, dir), parcelCell(false, true, dir),
]);

function browserifyCell(standalone) {
  return {
    name: `browserify${standalone ? "-standalone" : ""}`,
    family: "browserify",
    tool: () => tool("browserify"),
    loader: "node",
    source: "cjs",
    flags: `browserify ${standalone ? "--standalone matrix " : ""}--node (Babel modules=commonjs source)`,
    build: (ctx) => new Promise((done, fail) => {
      const browserify = require("browserify");
      const b = browserify(join(ctx.srcDir, "index.js"), { node: true, standalone: standalone ? "matrix" : undefined, basedir: ctx.srcDir });
      const files = new Set();
      b.on("dep", (row) => files.add(row.file ?? row.id));
      const out = createWriteStream(join(ctx.dir, "index.js"));
      b.bundle().on("error", fail).pipe(out);
      out.on("finish", () => done(files.size));
    }),
  };
}

export const browserify = [browserifyCell(false), browserifyCell(true)];
