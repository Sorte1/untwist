import { createRequire } from "node:module";
import { execFile, spawn, spawnSync } from "node:child_process";
import { promisify } from "node:util";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";


const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "..", "..");
const outRoot = join(repo, "fixtures/untranspile/matrix");
const work = join(here, ".work");
const fromTools = createRequire(join(here, "package.json"));

const rollup = fromTools("rollup");
const nodeResolve = fromTools("@rollup/plugin-node-resolve").default;
const commonjs = fromTools("@rollup/plugin-commonjs").default;
const terser = fromTools("terser");
const coffee = fromTools("coffeescript");
const canonicalizer = fromTools("esbuild-0.28");

function canonical(code) {
  try {
    return canonicalizer.transformSync(code, { target: "esnext", minifyWhitespace: true, charset: "utf8", legalComments: "none" }).code;
  } catch {
    return null;
  }
}

function babelLine(line) {
  const dir = join(here, "babel", line);
  const req = createRequire(join(dir, "package.json"));
  const six = line === "6";
  const core = req(six ? "babel-core" : "@babel/core");
  const version = (name) => req(`${name}/package.json`).version;
  const helpers = version(six ? "babel-helpers" : "@babel/helpers");
  const env = req.resolve(six ? "babel-preset-env" : "@babel/preset-env");
  const plugin = (name) => {
    try {
      return req.resolve(name);
    } catch {
      return null;
    }
  };
  return {
    dir,
    req,
    six,
    core,
    label: `${core.version}+helpers-${helpers}`,
    env,
    classProperties: plugin(six ? "babel-plugin-transform-class-properties" : "@babel/plugin-proposal-class-properties"),
    privateMethods: plugin("@babel/plugin-proposal-private-methods"),
    staticBlock: plugin("@babel/plugin-proposal-class-static-block"),
    objectRestSpread: six ? plugin("babel-plugin-transform-object-rest-spread") : null,
    runtime: six ? null : plugin("@babel/plugin-transform-runtime"),
  };
}

const babelLines = ["6", "7.0", "7.12", "7.20", "7.24", "7.28", "7.29", "8"].map((line) => [line, babelLine(line)]);

function babelOptions(b, { targets, loose = false, runtime = false }) {
  const plugins = [];
  if (b.classProperties) plugins.push([b.classProperties, b.six ? {} : { loose }]);
  if (b.privateMethods) plugins.push([b.privateMethods, { loose }]);
  if (b.staticBlock) plugins.push(b.staticBlock);
  if (b.objectRestSpread) plugins.push(b.objectRestSpread);
  if (runtime) plugins.push([b.runtime, b.core.version.startsWith("7.0") || b.core.version.startsWith("7.12") ? { useESModules: runtime === "esm" } : {}]);
  const envOptions = { targets, ...(loose ? { loose } : {}), ...(runtime === "esm" ? { modules: false } : {}) };
  return {
    presets: [[b.env, envOptions]],
    plugins,
    sourceType: "module",
    babelrc: false,
    ...(b.six ? {} : { configFile: false }),
    compact: false,
  };
}

function regeneratorPrelude(b, code) {
  if (!/\bregeneratorRuntime\b/.test(code) || /require\(|import /.test(code)) return code;
  if (/\bregeneratorRuntime = /.test(code)) return code;
  const path = b.req.resolve("regenerator-runtime/runtime.js");
  return readFileSync(path, "utf8") + "\n" + code;
}

async function bundle(code, dir, name) {
  const entryDir = join(dir, ".entry");
  mkdirSync(entryDir, { recursive: true });
  const entry = join(entryDir, `${name}.mjs`);
  writeFileSync(entry, code);
  const build = await rollup.rollup({
    input: entry,
    plugins: [nodeResolve({ rootDir: dir, browser: true }), commonjs()],
    onwarn: () => {},
  });
  const { output } = await build.generate({ format: "iife", name: undefined });
  await build.close();
  return output[0].code;
}

function babelVariant(line, b, name, opts, bundled) {
  return {
    tool: line === "6" ? "babel6" : "babel",
    version: line,
    options: name,
    label: b.label,
    async compile(source, feature) {
      const out = (b.core.transformSync ?? b.core.transform)(source, babelOptions(b, opts)).code;
      if (!opts.runtime) return regeneratorPrelude(b, out);
      return bundle(out, b.dir, `${line}-${name}-${feature}`);
    },
    bundled,
  };
}

const ie11 = { ie: "11" };
const babelVariants = babelLines.flatMap(([line, b]) => {
  const list = [babelVariant(line, b, "ie11", { targets: ie11 }), babelVariant(line, b, "ie11-loose", { targets: ie11, loose: true })];
  if (b.runtime) list.push(babelVariant(line, b, "ie11-runtime", { targets: ie11, runtime: "esm" }, true));
  if (line === "7.28" || line === "7.29" || line === "8") list.push(babelVariant(line, b, "chrome45", { targets: { chrome: "45" } }));
  if (line === "7.29" || line === "8") {
    list.push(babelVariant(line, b, "ie11-runtime-cjs", { targets: ie11, runtime: "cjs" }, true));
    list.push(babelVariant(line, b, "chrome58", { targets: { chrome: "58" } }));
    list.push(babelVariant(line, b, "esmodules", { targets: { esmodules: true } }));
  }
  return list;
});

const tsLines = ["3.9", "4.9", "5.0", "5.9", "6.0"].map((line) => [line, fromTools(`ts-${line}`)]);

function tsVariant(line, ts, name, target, extra = {}, helpers = false) {
  return {
    tool: "ts",
    version: line,
    options: name,
    label: ts.version,
    async compile(source, feature) {
      const compilerOptions = {
        target,
        module: helpers ? ts.ModuleKind.ESNext : ts.ModuleKind.None,
        removeComments: false,
        alwaysStrict: true,
        ignoreDeprecations: line === "6.0" ? "6.0" : line.startsWith("5") ? "5.0" : undefined,
        ...(helpers ? { importHelpers: true } : {}),
        ...extra,
      };
      const input = helpers ? source + "\nexport {};\n" : source;
      const out = ts.transpileModule(input, { compilerOptions, reportDiagnostics: false, fileName: `${feature}.ts` }).outputText;
      if (!helpers) return out;
      return bundle(out, here, `ts-${line}-${name}-${feature}`);
    },
    bundled: helpers,
  };
}

const tsVariants = tsLines.flatMap(([line, ts]) => {
  const T = ts.ScriptTarget;
  const list = [];
  if (line !== "6.0") list.push(tsVariant(line, ts, "es3", T.ES3));
  list.push(
    tsVariant(line, ts, "es5", T.ES5),
    tsVariant(line, ts, "es5-iter", T.ES5, { downlevelIteration: true }),
    tsVariant(line, ts, "es5-tslib", T.ES5, { downlevelIteration: true }, true),
    tsVariant(line, ts, "es2015", T.ES2015),
    tsVariant(line, ts, "es2015-define", T.ES2015, { useDefineForClassFields: true }),
    tsVariant(line, ts, "es2017", T.ES2017),
    tsVariant(line, ts, "es2019", T.ES2019),
  );
  return list;
});

function ts7Variant(target) {
  const bin = join(here, "node_modules/ts-7.0/bin/tsc");
  return {
    tool: "ts",
    version: "7.0",
    options: target,
    label: fromTools("ts-7.0/package.json").version,
    async compile(source, feature) {
      const dir = join(work, "ts7", `${target}-${feature}`);
      rmSync(dir, { recursive: true, force: true });
      mkdirSync(dir, { recursive: true });
      writeFileSync(join(dir, `${feature}.ts`), source);
      spawnSync(process.execPath, [bin, "--target", target, "--noCheck", "--skipLibCheck", "--outDir", join(dir, "out"), "--module", "preserve", "--moduleDetection", "legacy", "--alwaysStrict", join(dir, `${feature}.ts`)], { encoding: "utf8" });
      const outFile = join(dir, "out", `${feature}.js`);
      if (!existsSync(outFile)) throw new Error("tsc 7 emitted nothing");
      return readFileSync(outFile, "utf8");
    },
  };
}

const swcLines = ["1.2", "1.3", "1.7", "1.16"].map((line) => [line, fromTools(`swc-${line}`)]);

function swcVariant(line, swc, name, target, loose = false) {
  return {
    tool: "swc",
    version: line,
    options: name,
    label: fromTools(`swc-${line}/package.json`).version,
    async compile(source) {
      return swc.transformSync(source, {
        jsc: { target, loose, parser: { syntax: "ecmascript" }, externalHelpers: false },
        isModule: true,
        module: { type: "commonjs" },
        configFile: false,
        swcrc: false,
      }).code;
    },
  };
}

const swcVariants = swcLines.flatMap(([line, swc]) => [
  swcVariant(line, swc, "es5", "es5"),
  swcVariant(line, swc, "es5-loose", "es5", true),
  swcVariant(line, swc, "es2015", "es2015"),
  swcVariant(line, swc, "es2019", "es2019"),
]);

const esbuildLines = ["0.14", "0.17", "0.19", "0.21", "0.25", "0.28"].map((line) => [line, fromTools(`esbuild-${line}`)]);

function esbuildVariant(line, api, target, minify = false) {
  return {
    tool: "esbuild",
    version: line,
    options: minify ? `${target}-min` : target,
    label: api.version,
    async compile(source) {
      return api.transformSync(source, { target, charset: "utf8", minify }).code;
    },
  };
}

const esbuildVariants = esbuildLines.flatMap(([line, api]) => {
  const list = ["es2015", "es2017", "es2019", "es2020"].map((target) => esbuildVariant(line, api, target));
  if (line === "0.28") list.push(esbuildVariant(line, api, "es2015", true));
  return list;
});

const closureJars = [
  ["20230802", join(here, "node_modules/google-closure-compiler-java/compiler.jar")],
  ["20261109", join(here, "node_modules/closure-20261109/node_modules/google-closure-compiler-java/compiler.jar")],
];

function closureVariant(version, jar, out) {
  return {
    tool: "closure",
    version,
    options: `simple-${out.toLowerCase()}`,
    label: version,
    async compile(source, feature) {
      const dir = join(work, "closure");
      mkdirSync(dir, { recursive: true });
      const file = join(dir, `${version}-${out}-${feature}.js`);
      writeFileSync(file, source);
      const { stdout } = await promisify(execFile)("java", ["-jar", jar, "--compilation_level", "SIMPLE", "--language_in", "ECMASCRIPT_NEXT", "--language_out", out, "--js", basename(file), "--warning_level", "QUIET"], { encoding: "utf8", maxBuffer: 1 << 26, cwd: dir });
      return stdout;
    },
  };
}

const closureVariants = closureJars.flatMap(([version, jar]) => ["ECMASCRIPT5", "ECMASCRIPT_2015"].map((out) => closureVariant(version, jar, out)));

function bundled(variant) {
  return {
    ...variant,
    options: `${variant.options}-iife-terser`,
    async compile(source, feature) {
      const code = await variant.compile(source, feature);
      return (await terser.minify(`(function () {\n${code}\n})();`, { compress: true, mangle: true })).code;
    },
  };
}

function minified(variant) {
  return {
    ...variant,
    options: `${variant.options}-terser`,
    async compile(source, feature) {
      const code = await variant.compile(source, feature);
      return (await terser.minify(code, { compress: true, mangle: true })).code;
    },
  };
}

const safari9 = babelLines
  .filter(([line]) => line === "7.28" || line === "7.29")
  .map(([line, b]) => babelVariant(line, b, "safari9", { targets: { safari: "9" } }));

const latest = (tool, version, options) => [...babelVariants, ...tsVariants, ...swcVariants].find((v) => v.tool === tool && v.version === version && v.options === options);

const variants = [
  ...babelVariants,
  ...tsVariants,
  ...["es2015", "es2017", "es2019"].map(ts7Variant),
  ...swcVariants,
  ...esbuildVariants,
  ...closureVariants,
  minified(latest("babel", "7.29", "ie11")),
  minified(latest("babel", "7.29", "ie11-runtime")),
  minified(latest("babel", "7.29", "chrome45")),
  minified(latest("babel", "7.28", "chrome45")),
  minified(latest("babel", "7.28", "ie11")),
  ...safari9,
  ...safari9.map(bundled),
  minified(latest("babel", "7.29", "ie11-loose")),
  minified(latest("ts", "5.9", "es5")),
  minified(latest("ts", "5.9", "es5-tslib")),
  minified(latest("swc", "1.16", "es5")),
];

function run(code) {
  const dir = join(work, "run");
  mkdirSync(dir, { recursive: true });
  const file = join(dir, `p${createHash("sha1").update(code).digest("hex").slice(0, 12)}.js`);
  writeFileSync(file, code);
  return new Promise((resolve) => {
    const child = spawn(process.execPath, [file], { cwd: dir, stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    const timer = setTimeout(() => child.kill("SIGKILL"), 20000);
    child.stdout.on("data", (d) => (stdout += d));
    child.stderr.on("data", (d) => (stderr += d));
    child.on("close", (status) => {
      clearTimeout(timer);
      resolve(`${status}\n${stdout}\n${stderr.split("\n")[0] ?? ""}`);
    });
  });
}

const features = readdirSync(join(here, "features"))
  .filter((f) => f.endsWith(".js"))
  .sort()
  .map((f) => [basename(f, ".js"), readFileSync(join(here, "features", f), "utf8")]);

rmSync(outRoot, { recursive: true, force: true });
mkdirSync(outRoot, { recursive: true });
const manifest = [];
const byHash = new Map();
const expected = new Map(await Promise.all(features.map(async ([name, source]) => [name, await run(source)])));

function record(tool, version, options, feature, status, path = "-", faithful = "-") {
  manifest.push([tool, version, options, feature, status, path, faithful].join("\t"));
}

function keep(variant, feature, code, faithful) {
  const hash = createHash("sha256").update(code).digest("hex");
  const seen = byHash.get(hash);
  const { tool, version, options } = variant;
  if (seen) {
    record(tool, version, options, feature, "same", seen, faithful);
    return;
  }
  const rel = `${tool}-${version}/${options}/${feature}.js`;
  mkdirSync(join(outRoot, dirname(rel)), { recursive: true });
  writeFileSync(join(outRoot, rel), code.endsWith("\n") ? code : code + "\n");
  byHash.set(hash, rel);
  record(tool, version, options, feature, "written", rel, faithful);
}

async function cell(variant, feature, source) {
  let code;
  try {
    code = await variant.compile(source, feature);
  } catch (error) {
    return { status: "refused:" + String(error.message).split("\n")[0].replace(/\s+/g, " ").slice(0, 80) };
  }
  const printed = canonical(code);
  if (printed === null) return { status: "refused:the tool's output does not parse" };
  if (!variant.bundled && printed === canonical(source)) return { status: "native" };
  const faithful = (await run(code)) === expected.get(feature) ? "faithful" : "differs";
  return { code, faithful };
}

for (const variant of variants) {
  const key = `${variant.tool}-${variant.version}/${variant.options}`;
  const results = await Promise.all(features.map(([feature, source]) => cell(variant, feature, source)));
  features.forEach(([feature], at) => {
    const result = results[at];
    if (result.code === undefined) record(variant.tool, variant.version, variant.options, feature, result.status);
    else keep(variant, feature, result.code, result.faithful);
  });
  console.log(`done ${key}`);
}

const coffeeDir = join(here, "coffee");
for (const file of readdirSync(coffeeDir).filter((f) => f.endsWith(".coffee")).sort()) {
  const code = coffee.compile(readFileSync(join(coffeeDir, file), "utf8"));
  const variant = { tool: "coffee", version: coffee.VERSION, options: "default" };
  keep(variant, basename(file, ".coffee"), code, "faithful");
}

const labels = {};
for (const v of variants) labels[`${v.tool}-${v.version}`] = v.label;
labels[`coffee-${coffee.VERSION}`] = coffee.VERSION;
writeFileSync(join(outRoot, "VERSIONS.json"), JSON.stringify({ node: process.version, tools: labels }, null, 2) + "\n");
writeFileSync(join(outRoot, "MANIFEST"), "tool\tversion\toptions\tfeature\tstatus\tfile\tfaithful\n" + manifest.join("\n") + "\n");
console.log(`${byHash.size} files, ${manifest.length} cells`);
