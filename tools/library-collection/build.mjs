import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { owner, parseArgs } from "./common.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const require = createRequire(join(HERE, "package.json"));
const TOOLS = ["rollup", "@rollup/plugin-commonjs", "@rollup/plugin-json", "@rollup/plugin-node-resolve", "@rollup/plugin-replace", "esbuild", "webpack", "terser", "terser-webpack-plugin", "vite", "@jridgewell/trace-mapping"];
const BUNDLERS = ["rollup-cjs", "esbuild", "webpack5", "webpack5-prod", "vite8"];
const MINIFIED = { "rollup-cjs": ["none", "terser", "esbuild"], esbuild: ["none", "terser", "esbuild"], webpack5: ["none", "terser", "esbuild"], "webpack5-prod": ["terser"], vite8: ["oxc"] };
const MAX_SUBPATHS = 12;
const NODE_BUILTINS = ["fs", "path", "crypto", "stream", "http", "https", "url", "zlib", "util", "buffer", "os", "net", "tls", "assert", "vm", "child_process", "events", "string_decoder", "querystring", "punycode", "process", "worker_threads", "perf_hooks", "module", "dns", "readline", "tty", "constants", "timers", "async_hooks", "http2", "inspector", "dgram", "cluster", "v8"];
const DEFINE = { "process.env.NODE_ENV": JSON.stringify("production") };

const options = parseArgs(process.argv.slice(2));
const [installDir, name, version, outArg] = options._;
if (!installDir || !name || !version || !outArg) {
  console.error("usage: node build.mjs <install-dir> <package> <version> <out-dir> [--bundlers a,b]");
  process.exit(2);
}
const INSTALL = resolve(installDir);
const OUT = resolve(outArg);
const bundlers = options.bundlers ? options.bundlers.split(",") : BUNDLERS;
const toolVersion = (tool) => JSON.parse(readFileSync(join(HERE, "node_modules", tool, "package.json"), "utf8")).version;
const packageRoot = join(INSTALL, "node_modules", name);
const manifest = JSON.parse(readFileSync(join(packageRoot, "package.json"), "utf8"));

function packageVersions() {
  const found = {};
  const walk = (folder, prefix) => {
    if (!existsSync(folder)) return;
    for (const entry of readdirSync(folder, { withFileTypes: true })) {
      if (!entry.isDirectory() || entry.name.startsWith(".")) continue;
      if (entry.name.startsWith("@")) {
        for (const scoped of readdirSync(join(folder, entry.name), { withFileTypes: true })) {
          if (scoped.isDirectory()) visit(join(folder, entry.name, scoped.name), `${prefix}${entry.name}/${scoped.name}`);
        }
        continue;
      }
      visit(join(folder, entry.name), `${prefix}${entry.name}`);
    }
  };
  const visit = (dir, key) => {
    const file = join(dir, "package.json");
    if (!existsSync(file)) return;
    const data = JSON.parse(readFileSync(file, "utf8"));
    found[key] = `${data.name ?? key.split("node_modules/").pop()}@${data.version}`;
    walk(join(dir, "node_modules"), `${key}/node_modules/`);
  };
  walk(join(INSTALL, "node_modules"), "node_modules/");
  return found;
}

const PACKAGES = packageVersions();

function ownerOf(path) {
  const found = owner(path);
  if (!found) return null;
  const normal = path.replace(/\\/g, "/");
  const at = normal.indexOf("node_modules/");
  const chain = normal.slice(at, normal.lastIndexOf(found.file) - 1);
  const pinned = PACKAGES[chain] ?? PACKAGES[`node_modules/${found.name}`];
  if (!pinned) return null;
  return `npm:${pinned}:${found.file}`;
}

function subpaths() {
  const exports = manifest.exports;
  if (!exports || typeof exports !== "object" || Array.isArray(exports)) return [];
  return Object.keys(exports)
    .filter((key) => key.startsWith("./") && key !== "." && !key.includes("*") && !key.endsWith("/"))
    .filter((key) => !/package\.json$|\.css$|\.json$|\.d\.ts$|(^|\/)(node|server|internal|test|tests|bin|cli)(\/|$)/.test(key))
    .slice(0, MAX_SUBPATHS)
    .map((key) => `${name}/${key.slice(2)}`);
}

function entryText(specifiers) {
  const lines = specifiers.map((specifier, index) => `import * as m${index} from ${JSON.stringify(specifier)};`);
  lines.push(`globalThis.__lib = [${specifiers.map((_, index) => `m${index}`).join(", ")}];`);
  return lines.join("\n") + "\n";
}

function declarations(code) {
  const { parseAst } = require("rollup/parseAst");
  const found = [];
  let ast;
  try {
    ast = parseAst(code, { allowReturnOutsideFunction: true });
  } catch {
    return found;
  }
  const visit = (node) => {
    if (!node || typeof node.type !== "string") return;
    if (/^(Function|Class)(Declaration|Expression)$/.test(node.type) && node.id) found.push([node.id.name, node.start]);
    if (node.type === "VariableDeclarator" && node.id?.type === "Identifier" && node.init && /Function|Class/.test(node.init.type)) found.push([node.id.name, node.start]);
    for (const value of Object.values(node)) {
      if (Array.isArray(value)) value.forEach(visit);
      else if (value && typeof value === "object" && typeof value.type === "string") visit(value);
    }
  };
  visit(ast);
  return found;
}

function installedScript(path) {
  const found = owner(path);
  if (!found || !/\.(c|m)?js$/.test(found.file)) return false;
  const normal = path.replace(/\\/g, "/").replace(/^\0/, "").replace(/\?.*$/, "");
  const local = normal.slice(normal.indexOf("node_modules/"));
  return existsSync(join(INSTALL, local)) || existsSync(join(INSTALL, found.key));
}

function lineStarts(code) {
  const starts = [0];
  for (let i = 0; i < code.length; i++) if (code.charCodeAt(i) === 10) starts.push(i + 1);
  return starts;
}

function namesFromMap(code, map) {
  if (!map) return {};
  const { TraceMap, originalPositionFor } = require("@jridgewell/trace-mapping");
  const traced = new TraceMap(typeof map === "string" ? map : JSON.stringify(map));
  const starts = lineStarts(code);
  const lineOf = (offset) => {
    let low = 0;
    let high = starts.length - 1;
    while (low < high) {
      const middle = (low + high + 1) >> 1;
      if (starts[middle] <= offset) low = middle;
      else high = middle - 1;
    }
    return low;
  };
  const names = new Map();
  for (const [binding, offset] of declarations(code)) {
    const line = lineOf(offset);
    const position = originalPositionFor(traced, { line: line + 1, column: offset - starts[line] });
    const path = position.source?.replace(/^webpack:\/\/[^/]*\//, "") ?? "";
    const source = installedScript(path) ? ownerOf(path) : null;
    if (names.has(binding) && names.get(binding) !== source) names.set(binding, null);
    else names.set(binding, source);
  }
  return Object.fromEntries([...names].filter(([, value]) => value));
}

async function bundleRollup(entry) {
  const { rollup } = require("rollup");
  const commonjs = require("@rollup/plugin-commonjs");
  const { nodeResolve } = require("@rollup/plugin-node-resolve");
  const replace = require("@rollup/plugin-replace");
  const json = require("@rollup/plugin-json");
  const bundle = await rollup({
    input: entry,
    plugins: [replace({ preventAssignment: true, values: DEFINE }), nodeResolve({ preferBuiltins: false, browser: true, rootDir: INSTALL }), json(), commonjs({ strictRequires: true })],
    onwarn() {},
  });
  const { output } = await bundle.generate({ format: "iife", sourcemap: true, name: "__untwist" });
  return { code: output[0].code, map: output[0].map };
}

async function bundleEsbuild(entry) {
  const { build } = require("esbuild");
  const result = await build({ entryPoints: [entry], bundle: true, write: false, outfile: join(dirname(entry), "out.js"), format: "iife", platform: "browser", sourcemap: "external", logLevel: "silent", define: DEFINE, absWorkingDir: INSTALL, external: NODE_BUILTINS });
  const code = result.outputFiles.find((file) => file.path.endsWith(".js")).text;
  const map = result.outputFiles.find((file) => file.path.endsWith(".map")).text;
  return { code, map };
}

async function bundleWebpack(entry, production) {
  const webpack = require("webpack");
  const TerserPlugin = require("terser-webpack-plugin");
  const out = join(dirname(entry), production ? "webpack-prod" : "webpack");
  const fallback = Object.fromEntries(NODE_BUILTINS.map((builtin) => [builtin, false]));
  const optimization = production
    ? { minimize: true, concatenateModules: true, minimizer: [new TerserPlugin({ extractComments: false, terserOptions: { mangle: false } })] }
    : { minimize: false, moduleIds: "named", concatenateModules: false };
  const stats = await new Promise((done, fail) =>
    webpack(
      { mode: "production", context: INSTALL, entry, target: "web", devtool: "source-map", output: { path: out, filename: "out.js" }, optimization, performance: { hints: false }, resolve: { fallback } },
      (error, stats) => (error || stats.hasErrors() ? fail(error ?? new Error(stats.toString("errors-only"))) : done(stats)),
    ),
  );
  const complete = production ? {} : completeModules(stats.toJson({ all: false, modules: true, usedExports: true, providedExports: true, nestedModules: false }).modules ?? []);
  return { code: readFileSync(join(out, "out.js"), "utf8"), map: readFileSync(join(out, "out.js.map"), "utf8"), complete };
}

function exportTarget(value) {
  if (typeof value === "string") return value;
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  for (const condition of ["require", "node", "default"]) {
    const found = exportTarget(value[condition]);
    if (found) return found;
  }
  return null;
}

function importSpecifier(packageName, file) {
  let data;
  try {
    data = JSON.parse(readFileSync(join(INSTALL, "node_modules", packageName, "package.json"), "utf8"));
  } catch {
    return null;
  }
  if (data.exports === undefined) return `${packageName}/${file}`;
  const exports = typeof data.exports === "string" || Array.isArray(data.exports) || Object.keys(data.exports).some((key) => !key.startsWith(".")) ? { ".": data.exports } : data.exports;
  for (const [key, value] of Object.entries(exports)) {
    if (key.includes("*") || exportTarget(value) !== `./${file}`) continue;
    return key === "." ? packageName : `${packageName}/${key.slice(2)}`;
  }
  return null;
}

function completeModules(modules) {
  const complete = {};
  for (const module of modules) {
    const name = module.name ?? "";
    const found = /^\.\/node_modules\/((?:@[^/]+\/)?[^/]+)\/(.+\.c?js)$/.exec(name);
    if (!found || found[2].includes("node_modules/") || module.moduleType?.startsWith("javascript/esm")) continue;
    const provided = module.providedExports;
    const used = module.usedExports;
    const whole = used === true || (Array.isArray(used) && Array.isArray(provided) && provided.every((key) => used.includes(key)));
    if (!whole) continue;
    const specifier = importSpecifier(found[1], found[2]);
    if (specifier) complete[name] = specifier;
  }
  return complete;
}

async function bundleVite(entry) {
  const { build } = await import(require.resolve("vite"));
  const result = await build({
    root: dirname(entry),
    configFile: false,
    logLevel: "silent",
    define: DEFINE,
    resolve: { mainFields: ["browser", "module", "main"] },
    build: { write: false, minify: "oxc", sourcemap: true, modulePreload: false, rolldownOptions: { input: entry, output: { minify: { mangle: false, compress: true } } } },
  });
  const outputs = (Array.isArray(result) ? result : [result]).flatMap((item) => item.output);
  const chunk = outputs.find((item) => item.type === "chunk" && item.isEntry);
  return { code: chunk.code, map: chunk.map };
}

async function minify(code, minifier) {
  if (minifier === "none" || minifier === "oxc") return code;
  if (minifier === "terser") return (await require("terser").minify(code, { mangle: false, compress: true })).code;
  if (minifier === "esbuild") return (await require("esbuild").transform(code, { minifySyntax: true, minifyWhitespace: true, minifyIdentifiers: false })).code;
  throw new Error(`unknown minifier ${minifier}`);
}

async function bundle(bundler, entry) {
  if (bundler === "rollup-cjs") return bundleRollup(entry);
  if (bundler === "esbuild") return bundleEsbuild(entry);
  if (bundler === "webpack5") return bundleWebpack(entry, false);
  if (bundler === "webpack5-prod") return bundleWebpack(entry, true);
  if (bundler === "vite8") return bundleVite(entry);
  throw new Error(`unknown bundler ${bundler}`);
}

const sha = (text) => createHash("sha256").update(text).digest("hex").slice(0, 16);
const safe = `${name.replace("/", "__")}@${version}`;
const scratch = join(INSTALL, ".untwist-entry");
rmSync(scratch, { recursive: true, force: true });
mkdirSync(scratch, { recursive: true });
mkdirSync(OUT, { recursive: true });
const builds = [];
const failures = [];
const candidates = [[name, ...subpaths()], [name]];
for (const bundler of bundlers) {
  let bundled = null;
  for (const specifiers of candidates) {
    const entry = join(scratch, `${bundler}.entry.mjs`);
    writeFileSync(entry, entryText(specifiers));
    try {
      bundled = await bundle(bundler, entry);
      break;
    } catch (error) {
      failures.push(`${bundler} [${specifiers.length} entries]: ${String(error.message).split("\n")[0]}`);
    }
  }
  if (!bundled) continue;
  const names = namesFromMap(bundled.code, bundled.map);
  for (const minifier of MINIFIED[bundler]) {
    let code;
    try {
      code = await minify(bundled.code, minifier);
    } catch (error) {
      failures.push(`${bundler} ${minifier}: ${String(error.message).split("\n")[0]}`);
      continue;
    }
    const file = `${safe}.${bundler}.${minifier}.js`;
    writeFileSync(join(OUT, file), code.replace(/\n\/\/# sourceMappingURL=[^\n]*\s*$/, "\n"));
    builds.push({ file, version, dir: "", bundler, minifier, obfuscator: "none", names, complete: bundled.complete ?? {}, sha: sha(code) });
  }
}
rmSync(scratch, { recursive: true, force: true });
const tools = Object.fromEntries(TOOLS.map((tool) => [tool, toolVersion(tool)]));
const record = { layout: "node_modules", id: name, label: name, group: "collection", versions: [version], tools, packages: {}, owners: PACKAGES, builds, failures };
writeFileSync(join(OUT, `${safe}.manifest.json`), JSON.stringify(record, null, 1) + "\n");
console.error(`${name}@${version}: ${builds.length} builds${failures.length ? `, ${failures.length} failures` : ""}`);
if (!builds.length) process.exit(1);
