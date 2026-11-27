import { cpSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const require = createRequire(join(HERE, "package.json"));
const OUT = resolve(process.argv[2] ?? "");
if (!process.argv[2]) {
  console.error("usage: node runtimes.mjs <out-dir>");
  process.exit(2);
}
const SOURCE = join(OUT, "app");
mkdirSync(OUT, { recursive: true });
cpSync(join(HERE, "runtime"), SOURCE, { recursive: true });
const entry = join(SOURCE, "entry.mjs");

async function rollupBuild(strict, format) {
  const { rollup } = require("rollup");
  const commonjs = require("@rollup/plugin-commonjs");
  const json = require("@rollup/plugin-json");
  const bundle = await rollup({ input: entry, plugins: [json(), commonjs({ strictRequires: strict, transformMixedEsModules: true })], onwarn() {} });
  const { output } = await bundle.generate({ format, name: "app", inlineDynamicImports: true });
  return output[0].code;
}

async function esbuildBuild(format) {
  const result = await require("esbuild").build({ entryPoints: [entry], bundle: true, write: false, format, platform: "browser", logLevel: "silent" });
  return result.outputFiles[0].text;
}

async function webpackBuild(pkg, mode) {
  const webpack = require(pkg);
  const out = join(OUT, `dist-${pkg}-${mode}`);
  const optimization = mode === "production" ? { minimize: false } : {};
  await new Promise((done, fail) =>
    webpack({ mode, context: SOURCE, entry, target: "web", devtool: false, output: { path: out, filename: "main.js" }, optimization, performance: { hints: false } }, (error, stats) =>
      error || stats.hasErrors() ? fail(error ?? new Error(stats.toString("errors-only"))) : done(),
    ),
  );
  return readFileSync(join(out, "main.js"), "utf8");
}

async function viteBuild() {
  const { build } = await import(require.resolve("vite"));
  const result = await build({ configFile: false, logLevel: "silent", root: SOURCE, build: { write: false, minify: false, modulePreload: false, rolldownOptions: { input: entry } } });
  const outputs = (Array.isArray(result) ? result : [result]).flatMap((item) => item.output);
  return outputs.filter((item) => item.type === "chunk").map((item) => item.code).join("\n");
}

async function minify(code, minifier) {
  if (minifier === "terser") return (await require("terser").minify(code, { mangle: false, compress: true })).code;
  return (await require("esbuild").transform(code, { minifySyntax: true, minifyWhitespace: true, minifyIdentifiers: false })).code;
}

const builds = {
  "rollup-cjs.iife": () => rollupBuild(true, "iife"),
  "rollup-hoist.iife": () => rollupBuild(false, "iife"),
  "rollup-cjs.cjs": () => rollupBuild(true, "cjs"),
  "esbuild.iife": () => esbuildBuild("iife"),
  "esbuild.esm": () => esbuildBuild("esm"),
  "webpack5.none": () => webpackBuild("webpack", "none"),
  "webpack5.production": () => webpackBuild("webpack", "production"),
  "webpack5.development": () => webpackBuild("webpack", "development"),
  "webpack4.none": () => webpackBuild("webpack4", "none"),
  "webpack4.production": () => webpackBuild("webpack4", "production"),
  "vite8": () => viteBuild(),
};
let written = 0;
for (const [name, make] of Object.entries(builds)) {
  let code;
  try {
    code = await make();
  } catch (error) {
    console.error(`${name}: ${String(error.message).split("\n")[0]}`);
    continue;
  }
  for (const [suffix, text] of [["", code], [".terser", await minify(code, "terser")], [".esbuild", await minify(code, "esbuild")]]) {
    writeFileSync(join(OUT, `${name}${suffix}.js`), text);
    written++;
  }
}
console.error(`${written} runtime builds`);
