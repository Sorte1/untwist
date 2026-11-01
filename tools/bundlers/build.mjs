import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { copyFileSync, cpSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "../..");
const OUT = join(REPO, "fixtures/unpack-bundlers");
const APP = join(HERE, "app");
const ENTRY = join(APP, "index.js");
const WORK = join(HERE, "node_modules/.work");
const require = createRequire(join(HERE, "package.json"));
const manifest = [];
process.chdir(HERE);

const version = (name) => JSON.parse(readFileSync(require.resolve(`${name}/package.json`), "utf8")).version;
const sha256 = (file) => createHash("sha256").update(readFileSync(file)).digest("hex");

function record(file, bundler, loader, flags) {
  const text = readFileSync(file, "utf8");
  if (text.includes(REPO) || text.includes(HERE)) throw new Error(`${file} embeds a local path`);
  manifest.push([relative(OUT, file), bundler, loader, flags, sha256(file)].join("\t"));
}

function target(kind, name) {
  mkdirSync(join(OUT, kind), { recursive: true });
  return join(OUT, kind, name);
}

async function esbuild() {
  const { build } = require("esbuild");
  const banner = 'import { createRequire } from "module"; const require = createRequire(import.meta.url);';
  for (const format of ["iife", "cjs", "esm"]) {
    for (const minify of [false, true]) {
      const file = target("esbuild", `esbuild-${format}${minify ? "-min" : ""}.js`);
      await build({
        entryPoints: [relative(HERE, ENTRY)], absWorkingDir: HERE, bundle: true, format, minify, platform: "node",
        outfile: file, logLevel: "error", banner: format === "esm" ? { js: banner } : undefined,
      });
      const flags = `--bundle --platform=node --format=${format}${minify ? " --minify" : ""}${format === "esm" ? " --banner:js=<createRequire>" : ""}`;
      record(file, `esbuild@${version("esbuild")}`, "node", flags);
    }
  }
}

function bun() {
  let bunVersion;
  try {
    bunVersion = execFileSync("bun", ["--version"], { encoding: "utf8" }).trim();
  } catch {
    console.log("bun is not on PATH; the committed bun fixtures are kept");
    return;
  }
  for (const [name, args, loader] of [
    ["bun-node.js", ["--target=node", "--format=cjs"], "node"],
    ["bun-cjs.js", ["--target=bun", "--format=cjs"], "bun-cjs"],
  ]) {
    const file = target("bun", name);
    execFileSync("bun", ["build", relative(HERE, ENTRY), ...args, "--outfile", file], { cwd: HERE, stdio: "ignore" });
    record(file, `bun@${bunVersion}`, loader, `build ${args.join(" ")}`);
  }
}

async function rollup() {
  const { rollup } = require("rollup");
  const commonjs = require("@rollup/plugin-commonjs");
  const json = require("@rollup/plugin-json");
  const builds = [
    ["rollup", "rollup-iife.js", "iife", false, "node"],
    ["umd", "rollup-umd.js", "umd", false, "node"],
    ["systemjs", "rollup-system.js", "system", true, "systemjs"],
    ["amd", "rollup-amd.js", "amd", true, "amd"],
  ];
  for (const [kind, name, format, external, loader] of builds) {
    const bundle = await rollup({
      input: relative(HERE, ENTRY), external: external ? ["path"] : [],
      plugins: [commonjs({ strictRequires: true, ignore: external ? [] : ["path"] }), json()], onwarn: () => {},
    });
    const file = target(kind, name);
    const named = format === "iife" || format === "umd";
    await bundle.write({ file, format, name: named ? "app" : undefined, inlineDynamicImports: true, exports: "auto" });
    const flags = `--format=${format} commonjs(strictRequires, ${external ? "external path" : "ignore path"}) json inlineDynamicImports`;
    record(file, `rollup@${version("rollup")}`, loader, flags);
  }
}

async function vite() {
  const { build } = await import(require.resolve("vite"));
  const outDir = join(WORK, "vite");
  await build({
    configFile: false, logLevel: "error", root: APP,
    ssr: { target: "node", noExternal: true, external: ["path"] },
    build: { ssr: true, outDir, emptyOutDir: true, minify: false, lib: { entry: ENTRY, formats: ["iife"], name: "app" } },
  });
  const file = target("rollup", "vite-iife.js");
  copyFileSync(join(outDir, "index.js"), file);
  record(file, `vite@${version("vite")}`, "node", "build.lib formats=[iife] build.ssr ssr.noExternal ssr.external=[path] minify=false");
}

async function parcel() {
  const { Parcel } = require("@parcel/core");
  for (const scopeHoist of [true, false]) {
    const dir = scopeHoist ? "hoist" : "nohoist";
    const distDir = join(WORK, `parcel-${dir}`);
    const bundler = new Parcel({
      entries: ENTRY, defaultConfig: require.resolve("@parcel/config-default"), shouldDisableCache: true,
      cacheDir: join(WORK, "parcel-cache"), mode: "production", shouldContentHash: false, logLevel: "error",
      defaultTargetOptions: { shouldOptimize: false, shouldScopeHoist: scopeHoist, sourceMaps: false },
      targets: { app: { distDir, context: "node", outputFormat: "commonjs", engines: { node: ">= 22" }, includeNodeModules: false, isLibrary: false, optimize: false, scopeHoist, sourceMap: false } },
    });
    const result = await bundler.run();
    const files = result.bundleGraph.getBundles().map((b) => b.filePath).sort((a, b) => (a.endsWith("index.js") ? -1 : b.endsWith("index.js") ? 1 : a.localeCompare(b)));
    for (const built of files) {
      const file = target(join("parcel", dir), relative(distDir, built));
      copyFileSync(built, file);
      const loader = built.endsWith("index.js") ? "node" : "chunk";
      record(file, `parcel@${version("parcel")}`, loader, `context=node outputFormat=commonjs optimize=false scopeHoist=${scopeHoist}`);
    }
  }
}

async function parcelSingle() {
  const { Parcel } = require("@parcel/core");
  const app = join(WORK, "parcel-single-app");
  cpSync(APP, app, { recursive: true });
  const entry = join(app, "index.js");
  writeFileSync(entry, readFileSync(ENTRY, "utf8").split("\n").filter((line) => !line.includes('import("./lazy.js")')).join("\n"));
  for (const scopeHoist of [true, false]) {
    for (const optimize of [false, true]) {
      const name = `single-${scopeHoist ? "hoist" : "nohoist"}${optimize ? "-min" : ""}`;
      const distDir = join(WORK, `parcel-${name}`);
      const bundler = new Parcel({
        entries: entry, defaultConfig: require.resolve("@parcel/config-default"), shouldDisableCache: true,
        cacheDir: join(WORK, "parcel-cache"), mode: "production", shouldContentHash: false, logLevel: "error",
        defaultTargetOptions: { shouldOptimize: optimize, shouldScopeHoist: scopeHoist, sourceMaps: false },
        targets: { app: { distDir, context: "node", outputFormat: "commonjs", engines: { node: ">= 22" }, includeNodeModules: false, isLibrary: false, optimize, scopeHoist, sourceMap: false } },
      });
      const result = await bundler.run();
      const built = result.bundleGraph.getBundles().map((b) => b.filePath);
      if (built.length !== 1) throw new Error(`${name}: ${built.length} bundles, expected one`);
      const file = target("parcel", `${name}.js`);
      copyFileSync(built[0], file);
      record(file, `parcel@${version("parcel")}`, "node", `context=node outputFormat=commonjs optimize=${optimize} scopeHoist=${scopeHoist} app=no-lazy-chunk`);
    }
  }
}

async function metro() {
  const Metro = require("metro");
  const { getDefaultConfig, mergeConfig } = require("metro-config");
  const config = mergeConfig(await getDefaultConfig(HERE), {
    projectRoot: HERE, watchFolders: [HERE], cacheStores: [], resetCache: true, maxWorkers: 2,
    resolver: {
      resolveRequest: (context, name, platform) =>
        name === "path" ? { type: "sourceFile", filePath: join(HERE, "metro-node-path.js") } : context.resolveRequest(context, name, platform),
    },
    transformer: {
      babelTransformerPath: join(HERE, "metro-transformer.cjs"),
      getTransformOptions: async () => ({ transform: { experimentalImportSupport: true, inlineRequires: false } }),
    },
  });
  const file = target("metro", "metro.js");
  await Metro.runBuild(config, { entry: ENTRY, out: file, minify: false, dev: false, platform: "android", sourceMap: false });
  record(file, `metro@${version("metro")}`, "node", "runBuild dev=false minify=false experimentalImportSupport path->metro-node-path.js");
}

function webpack4() {
  const script = join(HERE, "webpack4.cjs");
  execFileSync(process.execPath, ["--openssl-legacy-provider", script, OUT], { cwd: HERE, stdio: "inherit" });
  const w = version("webpack");
  record(target("webpack", "webpack4-dev-paths.js"), `webpack@${w}`, "node", "mode=development devtool=false target=node maxChunks=1");
  record(target("webpack", "webpack4-externals.js"), `webpack@${w}`, "node", "mode=production minimize=false target=node externals={path:'commonjs path'} maxChunks=1");
}

function webpack5() {
  const script = join(HERE, "webpack5.cjs");
  const built = execFileSync(process.execPath, [script, OUT], { cwd: HERE, encoding: "utf8" }).trim();
  const w = `webpack@${built} (next@${version("next")})`;
  const minimized = "terser compress.passes=2 mangle comments=false wrap_func_args=false";
  record(target("webpack", "webpack5-dev.js"), w, "node", "mode=development devtool=false target=node maxChunks=1");
  record(target("webpack", "webpack5-prod.js"), w, "node", "mode=production minimize=false concatenateModules=false target=node maxChunks=1");
  record(target("webpack", "webpack5-prod-min.js"), w, "node", `mode=production concatenateModules=false target=node maxChunks=1 ${minimized}`);
  record(target("webpack", "webpack5-prod-concat.js"), w, "node", "mode=production minimize=false target=node externals={path:'commonjs path'} maxChunks=1");
  record(target("webpack", "webpack5-prod-concat-min.js"), w, "node", `mode=production target=node externals={path:'commonjs path'} maxChunks=1 ${minimized}`);
}

function nextApp() {
  const dir = join(WORK, "next");
  mkdirSync(join(dir, "pages"), { recursive: true });
  cpSync(APP, join(dir, "src"), { recursive: true });
  writeFileSync(join(dir, "package.json"), '{"name":"turbopack-fixture","private":true}\n');
  writeFileSync(join(dir, "next.config.js"), 'module.exports = { turbopack: { root: require("path").resolve(__dirname, "../../..") } };\n');
  writeFileSync(join(dir, "pages/index.js"), [
    'import { useEffect } from "react";',
    "export default function Page() {",
    '  useEffect(() => { import("../src/index.js"); }, []);',
    "  return null;",
    "}",
    'export async function getServerSideProps() { await import("../src/index.js"); return { props: {} }; }',
    "",
  ].join("\n"));
  return dir;
}

function turbopack() {
  const dir = nextApp();
  execFileSync(process.execPath, [require.resolve("next/dist/bin/next"), "build", "--turbopack"], {
    cwd: dir, stdio: "ignore", env: { ...process.env, NEXT_TELEMETRY_DISABLED: "1" },
  });
  const nextVersion = `next@${version("next")}`;
  const ssr = join(dir, ".next/server/chunks/ssr");
  const ours = readdirSync(ssr).filter((f) => f.endsWith(".js") && (f.includes("_src_") || f.startsWith("[externals]_path") || f === "[turbopack]_runtime.js")).sort();
  const entryChunk = ours.find((f) => readFileSync(join(ssr, f), "utf8").includes('"eval index"'));
  const loader = /Promise\.all\(\[([^\]]*)\]\.map\(\w+=>\w+\.l\(\w+\)\)\)\.then\(\(\)=>\w+\((\d+)\)\)/g;
  const found = readdirSync(ssr).filter((f) => f.endsWith(".js")).flatMap((f) => [...readFileSync(join(ssr, f), "utf8").matchAll(loader)])
    .find((m) => m[1].includes(entryChunk));
  if (!found) throw new Error("no loader names the app's turbopack server chunk");
  const loaded = [...ours.filter((f) => f.startsWith("[externals]")).map((f) => `"server/chunks/ssr/${f}"`), ...found[1].split(",")];
  const id = found[2];
  for (const f of ours) {
    const file = target("turbopack/server/server/chunks/ssr", f);
    copyFileSync(join(ssr, f), file);
    record(file, nextVersion, "chunk", "next build --turbopack, server (ssr) chunk");
  }
  const entry = target("turbopack/server", "index.js");
  const lines = ['var R = require("./server/chunks/ssr/[turbopack]_runtime.js")("server/pages/index.js");'];
  for (const f of loaded) lines.push(`R.c(${f});`);
  lines.push(`R.m(${id});`, "");
  writeFileSync(entry, lines.join("\n"));
  record(entry, "derived", "node", "Next's server page loader shape, naming the app's chunks and module");
  const browser = join(dir, ".next/static/chunks");
  const chunk = readdirSync(browser).filter((f) => f.endsWith(".js")).find((f) => readFileSync(join(browser, f), "utf8").includes('"eval cycle-b"'));
  const file = target("turbopack", "browser-chunk.js");
  copyFileSync(join(browser, chunk), file);
  record(file, nextVersion, "none", "next build --turbopack, browser chunk holding the app (no node loader)");
}

function handLoader(text) {
  if (text.startsWith("// @bun @bun-cjs")) return "bun-cjs";
  if (/^(var [^\n]*\n)?System\.register\(/.test(text)) return "systemjs";
  if (/^(var [^\n]*\n)?define\(/.test(text)) return "amd";
  return "node";
}

const nextRelease = () => `next@${version("next")}`;
const BROWSER_SHIM = ["derived", "node", "loads the chunk, then Turbopack's browser runtime, with the browser globals the runtime reads"];
const BROWSER_RUNTIME = () => [nextRelease(), "chunk", "next build --turbopack, browser runtime chunk (static/chunks/turbopack-*.js), copied"];
const SERVER_RUNTIME = () => [nextRelease(), "chunk", "next build --turbopack, server (ssr) runtime chunk, copied"];
const SERVER_LOADER = ["hand-written", "node", "Next's server page loader shape: the runtime, two server chunks, one module"];
const SERVER_LIB = ["hand-written", "chunk", "server chunk: two CommonJS modules"];
const HAND_DIRS = {
  "turbopack/min-entry": {
    "chunk.js": () => ["hand-written", "chunk", "browser chunk: a module push and a registration push naming two runtime entries"],
    "index.js": () => BROWSER_SHIM,
    "runtime.js": BROWSER_RUNTIME,
  },
  "turbopack/neg-entry": {
    "chunk.js": () => ["hand-written", "chunk", "refusal shape: an ESM module calling the runtime's esmImport/esmExport"],
    "index.js": () => BROWSER_SHIM,
    "runtime.js": BROWSER_RUNTIME,
  },
  "turbopack/min-server": {
    "index.js": () => SERVER_LOADER,
    "server/chunks/ssr/[turbopack]_runtime.js": SERVER_RUNTIME,
    "server/chunks/ssr/app.js": () => ["hand-written", "chunk", "server chunk: a strict arrow wrapper, cross-chunk requires, a value export, a cycle"],
    "server/chunks/ssr/lib.js": () => SERVER_LIB,
  },
  "turbopack/neg-server": {
    "index.js": () => SERVER_LOADER,
    "server/chunks/ssr/[turbopack]_runtime.js": SERVER_RUNTIME,
    "server/chunks/ssr/app.js": () => ["hand-written", "chunk", "refusal shape: a server chunk calling the runtime's esmImport/esmExport"],
    "server/chunks/ssr/lib.js": () => SERVER_LIB,
  },
};

function filesUnder(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((d) => (d.isDirectory() ? filesUnder(join(dir, d.name)) : [join(dir, d.name)]));
}

function handDirs() {
  for (const [dir, rows] of Object.entries(HAND_DIRS)) {
    const found = filesUnder(join(OUT, dir)).map((f) => relative(join(OUT, dir), f)).sort();
    const named = Object.keys(rows).sort();
    if (found.join("\n") !== named.join("\n")) throw new Error(`${dir}: files ${found} differ from the rows ${named}`);
    for (const name of named) record(join(OUT, dir, name), ...rows[name]());
  }
}

function hand() {
  handDirs();
  for (const kind of readdirSync(OUT, { withFileTypes: true }).filter((d) => d.isDirectory()).map((d) => d.name).sort()) {
    for (const f of readdirSync(join(OUT, kind)).filter((f) => /^(min|neg)-.*\.js$/.test(f)).sort()) {
      const file = join(OUT, kind, f);
      record(file, "hand-written", handLoader(readFileSync(file, "utf8")), f.startsWith("neg-") ? "refusal shape" : "smallest accepted shape");
    }
  }
}

const steps = { esbuild, bun, rollup, vite, parcel, parcelSingle, metro, webpack4, webpack5, turbopack, hand };
const only = process.argv.slice(2);
mkdirSync(WORK, { recursive: true });
for (const [name, step] of Object.entries(steps)) {
  if (only.length && !only.includes(name)) continue;
  console.log(`== ${name}`);
  await step();
}
if (!only.length) {
  const header = "# path\tbundler\tloader\tflags\tsha256\n";
  writeFileSync(join(OUT, "MANIFEST"), header + manifest.join("\n") + "\n");
  console.log(`MANIFEST: ${manifest.length} files`);
} else {
  console.log(manifest.join("\n"));
}
