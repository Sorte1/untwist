import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "../..");
const OUT = join(REPO, "fixtures/rollup-commonjs");
const WORK = join(HERE, "node_modules/.work");
const require = createRequire(join(HERE, "package.json"));
const manifest = [];
process.chdir(HERE);

const version = (name) => JSON.parse(readFileSync(join(HERE, "node_modules", name, "package.json"), "utf8")).version;
const sha256 = (text) => createHash("sha256").update(text).digest("hex");

const APPS = {
  synthetic: "apps/synthetic/index.js",
  esm: "apps/synthetic/esm.js",
  mixed: "apps/synthetic/mixed.js",
  "require-esm": "apps/synthetic/require-esm.js",
  corejs: "apps/corejs/index.js",
  polyfill: "apps/corejs/polyfill.js",
  npm: "apps/npm/index.js",
};

const MAJORS = [
  ["cjs11", "rollup2"],
  ["cjs15", "rollup2"],
  ["cjs17", "rollup2"],
  ["cjs20", "rollup2"],
  ["cjs21", "rollup2"],
  ["cjs22", "rollup2"],
  ["cjs24", "rollup3"],
  ["cjs25", "rollup4"],
  ["cjs26", "rollup4"],
  ["cjs28", "rollup4"],
  ["cjs29", "rollup4"],
];

const LATEST = "cjs29";
const STRICT_FROM = 22;

function printed(code) {
  const file = join(WORK, "run.js");
  writeFileSync(file, code);
  try {
    return execFileSync(process.execPath, [file], { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"], timeout: 20000 });
  } catch (error) {
    return `threw\n${error.stdout ?? ""}`;
  }
}

function record(name, code, row) {
  if (code.includes(REPO) || code.includes(HERE)) throw new Error(`${name} embeds a local path`);
  const file = join(OUT, name);
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, code);
  const stdout = printed(code);
  manifest.push([name, row.tool, row.options, row.post, row.app, stdout.startsWith("threw") ? "threw" : "ran", sha256(stdout).slice(0, 16)].join("\t"));
}

function major(plugin) {
  return Number(plugin.replace("cjs", ""));
}

async function rollupBuild(plugin, rollupName, app, options, format) {
  const { rollup } = require(rollupName);
  const commonjs = require(plugin);
  const nodeResolve = require("node-resolve");
  const bundle = await rollup({
    input: APPS[app],
    external: options.external ?? [],
    plugins: [(nodeResolve.nodeResolve ?? nodeResolve)(), (commonjs.default ?? commonjs)(options.commonjs ?? {})],
    onwarn: () => {},
  });
  const { output } = await bundle.generate({ format, name: format === "iife" ? "app" : undefined, exports: "auto" });
  await bundle.close?.();
  return output[0].code;
}

const SWC = () => require("@swc/core");
const OBFUSCATOR = () => require("javascript-obfuscator");

const POSTS = {
  plain: async (code) => code,
  terser: async (code) => (await require("terser").minify(code, { compress: true, mangle: true })).code,
  esbuild: async (code) => (await require("esbuild").transform(code, { minify: true, target: "es2020" })).code,
  swc: async (code) => (await SWC().minify(code, { compress: true, mangle: true })).code,
  "obf-default": async (code) => OBFUSCATOR().obfuscate(code, { seed: 1 }).getObfuscatedCode(),
  "obf-strings": async (code) =>
    OBFUSCATOR()
      .obfuscate(code, {
        seed: 1,
        stringArray: true,
        stringArrayThreshold: 1,
        stringArrayEncoding: ["base64"],
        stringArrayCallsTransform: true,
        stringArrayWrappersCount: 2,
        rotateStringArray: true,
        shuffleStringArray: true,
      })
      .getObfuscatedCode(),
  "obf-globals": async (code) =>
    OBFUSCATOR()
      .obfuscate(code, { seed: 1, renameGlobals: true, identifierNamesGenerator: "mangled-shuffled", stringArrayThreshold: 1 })
      .getObfuscatedCode(),
};

const ALL_POSTS = Object.keys(POSTS);
const SHORT_POSTS = ["plain", "terser", "obf-strings"];

async function emit(base, code, row, posts) {
  for (const post of posts) {
    const finished = await POSTS[post](code);
    record(`${base}--${post}.js`, finished, { ...row, post });
  }
}

const describe = (options) =>
  Object.keys(options.commonjs ?? {}).length === 0 && !options.external
    ? "defaults"
    : [
        ...Object.entries(options.commonjs ?? {}).map(([key, value]) => `${key}=${JSON.stringify(value)}`),
        ...(options.external ? [`external=${options.external.join(",")}`] : []),
      ].join(" ");

async function majors() {
  for (const [plugin, rollupName] of MAJORS) {
    const tool = `rollup@${version(rollupName)} @rollup/plugin-commonjs@${version(plugin)}`;
    for (const app of ["synthetic", "corejs", "npm"]) {
      const code = await rollupBuild(plugin, rollupName, app, {}, "iife");
      const posts = app === "npm" ? SHORT_POSTS : ALL_POSTS;
      await emit(`majors/v${major(plugin)}-${app}`, code, { tool, options: "defaults format=iife", app }, posts);
    }
  }
}

const OPTION_ROWS = [
  ["strict-true", "synthetic", { commonjs: { strictRequires: true } }],
  ["strict-false", "synthetic", { commonjs: { strictRequires: false } }],
  ["strict-auto", "synthetic", { commonjs: { strictRequires: "auto" } }],
  ["strict-debug", "synthetic", { commonjs: { strictRequires: "debug" } }],
  ["strict-list", "synthetic", { commonjs: { strictRequires: ["**/cycle-*.js", "**/lazy-*.js"] } }],
  ["strict-true-corejs", "corejs", { commonjs: { strictRequires: true } }],
  ["strict-false-corejs", "corejs", { commonjs: { strictRequires: false } }],
  ["strict-auto-polyfill", "polyfill", { commonjs: { strictRequires: "auto" } }],
  ["mixed", "mixed", { commonjs: { transformMixedEsModules: true } }],
  ["default-true", "esm", { commonjs: { defaultIsModuleExports: true } }],
  ["default-false", "esm", { commonjs: { defaultIsModuleExports: false } }],
  ["default-auto", "esm", { commonjs: { defaultIsModuleExports: "auto" } }],
  ["returns-false", "require-esm", { commonjs: { requireReturnsDefault: false } }],
  ["returns-true", "require-esm", { commonjs: { requireReturnsDefault: true } }],
  ["returns-auto", "require-esm", { commonjs: { requireReturnsDefault: "auto" } }],
  ["returns-preferred", "require-esm", { commonjs: { requireReturnsDefault: "preferred" } }],
  ["returns-namespace", "require-esm", { commonjs: { requireReturnsDefault: "namespace" } }],
  ["dynamic-ignored", "synthetic", { commonjs: { ignoreDynamicRequires: true } }],
];

async function options() {
  for (const plugin of ["cjs22", LATEST]) {
    const rollupName = plugin === LATEST ? "rollup4" : "rollup2";
    const tool = `rollup@${version(rollupName)} @rollup/plugin-commonjs@${version(plugin)}`;
    for (const [name, app, options] of OPTION_ROWS) {
      if (major(plugin) < STRICT_FROM && "strictRequires" in (options.commonjs ?? {})) continue;
      const code = await rollupBuild(plugin, rollupName, app, options, "iife");
      await emit(`options/v${major(plugin)}-${name}`, code, { tool, options: `${describe(options)} format=iife`, app }, SHORT_POSTS);
    }
    for (const esmExternals of [true, false]) {
      const options = { commonjs: { esmExternals }, external: ["ms"] };
      const code = await rollupBuild(plugin, rollupName, "npm", options, "cjs");
      await emit(`options/v${major(plugin)}-esm-externals-${esmExternals}`, code, { tool, options: `${describe(options)} format=cjs`, app: "npm" }, ["plain"]);
    }
  }
}

async function formats() {
  const tool = `rollup@${version("rollup4")} @rollup/plugin-commonjs@${version(LATEST)}`;
  for (const format of ["cjs", "es"]) {
    for (const app of ["synthetic", "corejs"]) {
      const code = await rollupBuild(LATEST, "rollup4", app, {}, format);
      await emit(`formats/${format}-${app}`, code, { tool, options: `defaults format=${format}`, app }, SHORT_POSTS);
    }
  }
}

async function viteBuild(name, app, kind, minify) {
  const { build } = await import(pathToFileURL(join(HERE, "node_modules", name, "dist/node/index.js")).href);
  const outDir = join(WORK, `${name}-${app}-${kind}-${minify}`);
  const entry = resolve(HERE, APPS[app]);
  const common = { configFile: false, logLevel: "silent", root: dirname(entry) };
  const commonjsOptions = { include: [/apps/, /node_modules/] };
  if (kind === "lib") {
    await build({
      ...common,
      build: { outDir, emptyOutDir: true, minify, commonjsOptions, lib: { entry, formats: ["iife"], name: "app", fileName: () => "out.js" } },
    });
    return readFileSync(join(outDir, "out.js"), "utf8");
  }
  const html = join(dirname(entry), `${app}.html`);
  writeFileSync(html, `<!doctype html><script type="module" src="./${relative(dirname(entry), entry)}"></script>\n`);
  try {
    await build({
      ...common,
      build: { outDir, emptyOutDir: true, minify, commonjsOptions, modulePreload: false, rollupOptions: { input: html } },
    });
  } finally {
    rmSync(html);
  }
  const assets = join(outDir, "assets");
  const [file] = readdirSync(assets).filter((f) => f.endsWith(".js"));
  return readFileSync(join(assets, file), "utf8");
}

async function vites() {
  for (const name of ["vite5", "vite6", "vite7", "vite8"]) {
    const tool = `vite@${version(name)}`;
    for (const [kind, minify] of [["lib", false], ["lib", true], ["app", false], ["app", true]]) {
      for (const app of ["synthetic", "corejs"]) {
        const code = await viteBuild(name, app, kind, minify);
        const minifier = minify ? (name === "vite8" ? "oxc" : "esbuild") : "false";
        await emit(`vite/${name}-${kind}-${app}${minify ? "-min" : ""}`, code, { tool, options: `build ${kind === "lib" ? "lib formats=[iife]" : "app index.html"} minify=${minifier} commonjsOptions.include=[apps,node_modules]`, app }, ["plain"]);
      }
    }
  }
}

async function rolldowns() {
  const { rolldown } = require("rolldown");
  const tool = `rolldown@${version("rolldown")}`;
  for (const format of ["iife", "cjs", "esm"]) {
    for (const minify of [false, true]) {
      for (const app of ["synthetic", "corejs"]) {
        const bundle = await rolldown({ input: APPS[app], platform: "neutral", resolve: { mainFields: ["main"] }, logLevel: "silent" });
        const { output } = await bundle.generate({ format, minify, name: format === "iife" ? "app" : undefined });
        await bundle.close();
        const posts = minify ? ["plain"] : SHORT_POSTS;
        await emit(`rolldown/${format}-${app}${minify ? "-min" : ""}`, output[0].code, { tool, options: `format=${format} minify=${minify}`, app }, posts);
      }
    }
  }
}

function licenses() {
  const dir = join(OUT, "licenses");
  mkdirSync(dir, { recursive: true });
  for (const name of ["core-js", "ms", "object-assign"]) {
    const root = dirname(require.resolve(`${name}/package.json`));
    const license = readdirSync(root).find((f) => /^licen[cs]e/i.test(f));
    copyFileSync(join(root, license), join(dir, `${name}-LICENSE`));
  }
}

mkdirSync(WORK, { recursive: true });
const only = new Set(process.argv.slice(2));
const groups = { majors, options, formats, vites, rolldowns };
for (const [name, run] of Object.entries(groups)) {
  if (only.size === 0 || only.has(name)) await run();
}
licenses();
const header = "# path\ttool\toptions\tpost\tapp\tnode\tstdout-sha256-16";
const manifestFile = join(OUT, "MANIFEST");
const previous = only.size === 0 || !existsSync(manifestFile) ? [] : readFileSync(join(OUT, "MANIFEST"), "utf8").split("\n").slice(1).filter(Boolean)
  .filter((line) => !manifest.some((row) => row.split("\t")[0] === line.split("\t")[0]));
writeFileSync(join(OUT, "MANIFEST"), [header, ...[...previous, ...manifest].sort()].join("\n") + "\n");
console.log(`${manifest.length} fixtures`);
