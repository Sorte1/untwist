import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { cpSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "../..");
const require = createRequire(join(HERE, "package.json"));
const MODULES = join(HERE, "node_modules");
const WORK = join(MODULES, ".work", "library-fixtures");
const OUT = join(REPO, "fixtures/libraries");

const version = (name) => JSON.parse(readFileSync(join(MODULES, name, "package.json"), "utf8")).version;
const sha256 = (text) => createHash("sha256").update(text).digest("hex");
const manifest = [];

function runs(text) {
  const probe = join(WORK, "probe.js");
  mkdirSync(WORK, { recursive: true });
  writeFileSync(probe, text);
  try {
    execFileSync(process.execPath, [probe], { stdio: "ignore", timeout: 10000 });
    return true;
  } catch {
    return false;
  }
}

function record(library, name, text, tools, flags) {
  if (text.includes(REPO) || text.includes(HERE)) throw new Error(`${name} embeds a local path`);
  if (!runs(text)) {
    console.log(`skipped ${library}/${name}: the tool's output does not run in node`);
    return;
  }
  const file = join(OUT, library, name);
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, text);
  manifest.push([`${library}/${name}`, tools, flags, sha256(text)].join("\t"));
}

function obfuscate(code) {
  const { obfuscate } = require("javascript-obfuscator");
  return obfuscate(code, {
    compact: true,
    identifierNamesGenerator: "mangled",
    renameGlobals: false,
    stringArray: true,
    stringArrayRotate: true,
    stringArrayShuffle: true,
    stringArrayThreshold: 1,
    stringArrayIndexShift: true,
    stringArrayWrappersCount: 1,
    seed: 7,
  }).getObfuscatedCode();
}

async function minify(code, minifier) {
  if (minifier === "terser") return (await require("terser").minify(code, { toplevel: true })).code;
  if (minifier === "esbuild") return (await require("esbuild").transform(code, { minify: true })).code;
  if (minifier === "swc") return (await require("@swc/core").minify(code, { compress: true, mangle: { topLevel: true } })).code;
  throw new Error(minifier);
}

function app(library, files) {
  const dir = join(WORK, library, "app");
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  for (const [name, text] of Object.entries(files)) writeFileSync(join(dir, name), text);
  return dir;
}

async function rollupBundle(entry, babelOptions) {
  const { rollup } = require("rollup");
  const commonjs = require("@rollup/plugin-commonjs");
  const { nodeResolve } = require("@rollup/plugin-node-resolve");
  const plugins = [nodeResolve(), commonjs({ strictRequires: true })];
  if (babelOptions) plugins.splice(1, 0, require("@rollup/plugin-babel").babel({ babelHelpers: babelOptions.plugins ? "runtime" : "bundled", babelrc: false, configFile: false, exclude: /node_modules[\\/](?!\.work)/, ...babelOptions }));
  const bundle = await rollup({ input: entry, plugins, onwarn() {} });
  return (await bundle.generate({ format: "iife", name: "app" })).output[0].code;
}

async function webpackBundle(entry, pkg, mode, babelOptions) {
  const webpack = require(pkg);
  const out = join(dirname(entry), `dist-${pkg}-${mode}`);
  const rules = babelOptions ? [{ test: /\.js$/, exclude: /node_modules[\\/](?!\.work)/, use: { loader: require.resolve(pkg === "webpack4" ? "babel-loader8" : "babel-loader"), options: { babelrc: false, configFile: false, ...babelOptions } } }] : [];
  await new Promise((done, fail) =>
    webpack({ mode, context: dirname(entry), entry, target: "web", devtool: false, output: { path: out, filename: "main.js" }, module: { rules }, performance: { hints: false } }, (error, stats) =>
      error || stats.hasErrors() ? fail(error ?? new Error(stats.toString("errors-only"))) : done(),
    ),
  );
  return readFileSync(join(out, "main.js"), "utf8");
}

async function esbuildBundle(entry, minify) {
  const result = await require("esbuild").build({ entryPoints: [entry], bundle: true, write: false, format: "iife", minify, logLevel: "silent" });
  return result.outputFiles[0].text;
}

async function viteBundle(entry, minify) {
  const { build } = await import(require.resolve("vite"));
  const outDir = join(dirname(entry), `dist-vite-${minify}`);
  await build({ configFile: false, logLevel: "error", root: dirname(entry), build: { outDir, emptyOutDir: true, minify, lib: { entry, formats: ["iife"], name: "app", fileName: () => "main.js" } } });
  return readFileSync(join(outDir, "main.js"), "utf8");
}

async function parcelBundle(entry, optimize) {
  const { Parcel } = require("@parcel/core");
  const distDir = join(dirname(entry), `dist-parcel-${optimize}`);
  const bundler = new Parcel({
    entries: entry, defaultConfig: require.resolve("@parcel/config-default"), shouldDisableCache: true, cacheDir: join(WORK, "parcel-cache"),
    mode: "production", shouldContentHash: false, logLevel: "error",
    defaultTargetOptions: { shouldOptimize: optimize, sourceMaps: false },
    targets: { app: { distDir, context: "browser", outputFormat: "global", isLibrary: false, optimize, sourceMap: false } },
  });
  const result = await bundler.run();
  const [bundle] = result.bundleGraph.getBundles();
  return readFileSync(bundle.filePath, "utf8");
}

const v = { rollup: `rollup@${version("rollup")} @rollup/plugin-commonjs@${version("@rollup/plugin-commonjs")}`, terser: `terser@${version("terser")}`, esbuild: `esbuild@${version("esbuild")}`, swc: `@swc/core@${version("@swc/core")}`, obf: `javascript-obfuscator@${version("javascript-obfuscator")}` };

async function variants(library, base, entry, babel) {
  const plain = await rollupBundle(entry, babel);
  record(library, `${base}rollup-cjs.js`, plain, v.rollup, "strictRequires=true format=iife");
  for (const minifier of ["terser", "esbuild", "swc"]) {
    const code = await minify(plain, minifier);
    record(library, `${base}rollup-cjs.${minifier}.js`, code, `${v.rollup} ${v[minifier]}`, `strictRequires=true format=iife, then ${minifier} minify`);
    if (minifier === "terser") record(library, `${base}rollup-cjs.terser.obf.js`, obfuscate(code), `${v.rollup} ${v.terser} ${v.obf}`, "then javascript-obfuscator stringArray rotate shuffle indexShift wrappers=1 mangled names");
  }
  for (const [pkg, label] of [["webpack", "webpack5"], ["webpack4", "webpack4"]]) {
    const prod = await webpackBundle(entry, pkg, "production", babel);
    record(library, `${base}${label}.prod.js`, prod, `webpack@${version(pkg)}`, "mode=production");
    if (label === "webpack5") record(library, `${base}${label}.prod.obf.js`, obfuscate(prod), `webpack@${version(pkg)} ${v.obf}`, "mode=production, then javascript-obfuscator");
    record(library, `${base}${label}.none.js`, await webpackBundle(entry, pkg, "none", babel), `webpack@${version(pkg)}`, "mode=none");
  }
  if (babel) return;
  record(library, `${base}esbuild.js`, await esbuildBundle(entry, false), v.esbuild, "--bundle --format=iife");
  const esmin = await esbuildBundle(entry, true);
  record(library, `${base}esbuild.min.js`, esmin, v.esbuild, "--bundle --format=iife --minify");
  record(library, `${base}esbuild.min.obf.js`, obfuscate(esmin), `${v.esbuild} ${v.obf}`, "--minify, then javascript-obfuscator");
  for (const minify of [false, "terser", "esbuild"]) {
    record(library, `${base}vite.${minify || "none"}.js`, await viteBundle(entry, minify), `vite@${version("vite")}`, `build.lib formats=[iife] minify=${minify}`);
  }
  for (const optimize of [false, true]) {
    record(library, `${base}parcel.${optimize ? "min" : "none"}.js`, await parcelBundle(entry, optimize), `parcel@${version("parcel")}`, `outputFormat=global optimize=${optimize}`);
  }
}

const CORE_JS_APP = `const sym = Symbol("s");
const keys = Reflect.ownKeys({ a: 1, [sym]: 2 });
console.log(keys.length, typeof keys[1]);
console.log([1, 2, 3].at(-1), Object.fromEntries([["x", 1]]).x, "abc".padStart(5, "-"));
console.log(Array.from(new Set([1, 1, 2])).length, [..."ab"].includes("b"), Object.getOwnPropertyNames(Symbol).length > 0);
Promise.resolve(4).then((value) => console.log("promise", value));
`;

async function coreJs() {
  const dir = "core-js";
  const polyfills = ["actual/reflect/own-keys", "actual/symbol", "actual/array/at", "actual/object/from-entries", "actual/string/pad-start", "actual/array/from", "actual/set", "actual/array/includes", "actual/promise"];
  const explicit = app("core-js", {
    "index.js": polyfills.map((path) => `require("${dir}/${path}");`).join("\n") + "\n" + CORE_JS_APP,
  });
  await variants("core-js", "", join(explicit, "index.js"));
  const preset = (useBuiltIns) => ({ presets: [[require.resolve("@babel/preset-env"), { targets: "ie 11", useBuiltIns, corejs: "3.39", modules: false }]] });
  const usage = app("core-js-usage", { "index.js": CORE_JS_APP });
  await variants("core-js", "babel-usage.", join(usage, "index.js"), preset("usage"));
  const entry = app("core-js-entry", { "index.js": `import "core-js/actual/reflect";\nimport "core-js/actual/array/at";\nimport "core-js/actual/object/from-entries";\n${CORE_JS_APP}` });
  await variants("core-js", "babel-entry.", join(entry, "index.js"), preset("entry"));
}

const HELPERS_APP = `class Animal {
  constructor(name) { this.name = name; }
  speak() { return \`\${this.name} speaks\`; }
  static of(name) { return new this(name); }
}
class Dog extends Animal {
  speak() { return super.speak() + " loudly"; }
}
function* count() { yield 1; yield 2; }
async function main() {
  const parts = [...[1, 2], 3];
  const { a, ...rest } = { a: 1, b: 2, c: 3 };
  let total = 0;
  for (const value of parts) total += await Promise.resolve(value);
  console.log(Dog.of("rex").speak(), parts.length, a, Object.keys(rest).length, total, [...count()].join(","));
}
main();
`;

const babelEnv = { presets: [[require.resolve("@babel/preset-env"), { targets: "ie 11", modules: false }]] };

async function babelInline() {
  const dir = app("babel-inline", { "index.js": HELPERS_APP });
  await variants("babel-helpers", "inline.", join(dir, "index.js"), babelEnv);
}

async function babelRuntime() {
  const dir = app("babel-runtime", { "index.js": HELPERS_APP });
  const options = { ...babelEnv, plugins: [[require.resolve("@babel/plugin-transform-runtime"), { corejs: false, helpers: true, regenerator: true }]] };
  await variants("babel-runtime", "", join(dir, "index.js"), options);
}

async function swcHelpers() {
  for (const [base, externalHelpers] of [["inline.", false], ["", true]]) {
    const { code } = await require("@swc/core").transform(HELPERS_APP, { jsc: { target: "es5", externalHelpers, parser: { syntax: "ecmascript" } }, module: { type: "es6" }, isModule: true });
    const dir = app(`swc-${base || "external"}`, { "index.js": code });
    await variants(externalHelpers ? "swc-helpers" : "swc-inline", base, join(dir, "index.js"));
  }
}

const TSLIB_APP = `import { __extends, __assign, __awaiter, __generator, __spreadArray, __rest } from "tslib";
var Base = (function () { function Base(name) { this.name = name; } Base.prototype.hello = function () { return "hi " + this.name; }; return Base; })();
var Child = (function (_super) { __extends(Child, _super); function Child() { return _super !== null && _super.apply(this, arguments) || this; } return Child; })(Base);
function main() {
  return __awaiter(this, void 0, void 0, function () {
    var merged, rest, value;
    return __generator(this, function (_a) {
      switch (_a.label) {
        case 0:
          merged = __assign({ a: 1 }, { b: 2 });
          rest = __rest(merged, ["a"]);
          return [4, Promise.resolve(__spreadArray([1], [2, 3], false).length)];
        case 1:
          value = _a.sent();
          console.log(new Child("ts").hello(), Object.keys(rest).length, value);
          return [2];
      }
    });
  });
}
main();
`;

async function tslib() {
  const dir = app("tslib", { "index.js": TSLIB_APP });
  await variants("tslib", "", join(dir, "index.js"));
}

const REGENERATOR_APP = `require("regenerator-runtime/runtime");
var marked = regeneratorRuntime.mark(function numbers() {
  return regeneratorRuntime.wrap(function (context) {
    while (1) switch (context.prev = context.next) {
      case 0: context.next = 2; return 1;
      case 2: context.next = 4; return 2;
      case 4: case "end": return context.stop();
    }
  }, numbers);
});
console.log(Array.from(marked()).join(","), typeof regeneratorRuntime.async);
`;

async function regenerator() {
  const dir = app("regenerator-runtime", { "index.js": REGENERATOR_APP });
  await variants("regenerator-runtime", "", join(dir, "index.js"));
}

const libraries = { "core-js": coreJs, "babel-helpers": babelInline, "babel-runtime": babelRuntime, "swc-helpers": swcHelpers, tslib, "regenerator-runtime": regenerator };
rmSync(OUT, { recursive: true, force: true });
for (const build of Object.values(libraries)) await build();
const rows = [...manifest].sort();
writeFileSync(join(OUT, "MANIFEST"), `# file\ttools\tflags\tsha256 (generated by tools/library-fingerprints/fixtures.mjs)\n${rows.join("\n")}\n`);
console.log(`${manifest.length} fixtures`);
