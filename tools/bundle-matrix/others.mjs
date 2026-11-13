import { execFileSync } from "node:child_process";
import { copyFileSync, createWriteStream, mkdirSync } from "node:fs";
import { join, relative } from "node:path";

import { WORK, bun, fresh, jsFiles, require, tool, version } from "./lib.mjs";

const ESBUILD = ["esbuild-0.14", "esbuild-0.17", "esbuild-0.19", "esbuild-0.21", "esbuild-0.24", "esbuild-0.25", "esbuild"];
const CREATE_REQUIRE = 'import { createRequire } from "module"; const require = createRequire(import.meta.url);';

function esbuildCell(alias, format, { minify = false, split = false, keepNames = false, target, platform = "node" } = {}) {
  const short = alias === "esbuild" ? version("esbuild").split(".").slice(0, 2).join(".") : alias.slice("esbuild-".length);
  const parts = [`esbuild${short}`, format, split && "split", minify && "min", keepNames && "keepnames", target, platform === "browser" && "browser"];
  return {
    name: parts.filter(Boolean).join("-"),
    family: "esbuild",
    tool: () => tool("esbuild", alias),
    loader: "node",
    flags: [`--bundle --platform=${platform} --format=${format}`, split && "--splitting", minify && "--minify", keepNames && "--keep-names", target && `--target=${target}`]
      .filter(Boolean).join(" "),
    build: async (ctx) => {
      const { build } = require(alias);
      const result = await build({
        entryPoints: { index: join(ctx.srcDir, "index.js") }, bundle: true, format, minify, splitting: split, keepNames,
        platform, target, outdir: ctx.dir, chunkNames: "[name]-[hash].chunk", logLevel: "silent", metafile: true,
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
];

function bunCell(format, extra = []) {
  const label = extra.map((a) => a.replace(/^--/, "").replace(/=.*/, "")).join("-");
  return {
    name: ["bun", format, label].filter(Boolean).join("-"),
    family: "esbuild",
    tool: () => `bun@${bun()}`,
    loader: "node",
    flags: `build --target=node --format=${format} ${extra.join(" ")}`.trim(),
    available: () => bun() !== null,
    build: async (ctx) => {
      const args = ["build", join(ctx.srcDir, "index.js"), "--target=node", `--format=${format}`, "--outdir", ctx.dir, "--entry-naming=[name].js", "--chunk-naming=[name]-[hash].chunk.js", ...extra];
      execFileSync("bun", args, { cwd: ctx.srcDir, stdio: "ignore" });
      return null;
    },
  };
}

export const buns = [bunCell("cjs"), bunCell("esm"), bunCell("esm", ["--splitting"]), bunCell("iife", ["--minify"])];

function parcelCell(scopeHoist, optimize) {
  return {
    name: `parcel-${scopeHoist ? "hoist" : "nohoist"}${optimize ? "-min" : ""}`,
    family: "parcel",
    tool: () => tool("parcel"),
    loader: "node",
    flags: `context=node outputFormat=commonjs optimize=${optimize} scopeHoist=${scopeHoist}`,
    build: async (ctx) => {
      const { Parcel } = require("@parcel/core");
      const distDir = fresh(join(WORK, "parcel-out"));
      const bundler = new Parcel({
        entries: join(ctx.srcDir, "index.js"), defaultConfig: require.resolve("@parcel/config-default"), shouldDisableCache: true,
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

export const parcel = [parcelCell(true, false), parcelCell(false, false), parcelCell(true, true), parcelCell(false, true)];

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
