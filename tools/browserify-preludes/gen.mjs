import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT = join(HERE, "../../fixtures/browserify-preludes");
const require = createRequire(join(HERE, "package.json"));
const version = (name) => JSON.parse(readFileSync(join(HERE, "node_modules", name, "package.json"), "utf8")).version;

const PACKS = ["browser-pack-0.9", "browser-pack-2", "browser-pack-3", "browser-pack-5", "browser-pack-6.0", "browser-pack-6.1"];
const STANDALONE = new Set(["browser-pack-3", "browser-pack-5", "browser-pack-6.0", "browser-pack-6.1"]);

const MINIFIERS = {
  terser: (code) => require("terser").minify(code, { compress: { passes: 2 }, mangle: true, format: { comments: false } }).then((r) => r.code),
  uglify: (code) => {
    const result = require("uglify-js").minify(code, { compress: { passes: 2 }, mangle: true });
    if (result.error) throw result.error;
    return result.code;
  },
  "esbuild-min": async (code) => (await require("esbuild").transform(code, { minify: true, loader: "js" })).code,
  "swc-min": async (code) => (await require("@swc/core").minify(code, { compress: true, mangle: true })).code,
};
const MINIFIER_TOOLS = { terser: "terser", uglify: "uglify-js", "esbuild-min": "esbuild", "swc-min": "@swc/core" };

function rows(entries) {
  const browserify = require("browserify");
  const b = browserify(entries.map((e) => join(HERE, "app", e)), { basedir: join(HERE, "app") });
  const collected = [];
  const through = require("through2");
  b.pipeline.get("pack").splice(0, 1, through.obj((row, _enc, next) => {
    collected.push({ id: row.id, source: row.source, deps: row.deps, entry: row.entry, order: row.order });
    next();
  }));
  return new Promise((done, fail) => {
    const stream = b.bundle();
    stream.on("error", fail);
    stream.on("data", () => {});
    stream.on("end", () => done(collected));
  });
}

function pack(name, collected, opts) {
  const bpack = require(name);
  const stream = bpack(opts);
  const chunks = [];
  return new Promise((done, fail) => {
    stream.on("data", (chunk) => chunks.push(Buffer.from(chunk)));
    stream.on("error", fail);
    stream.on("end", () => done(Buffer.concat(chunks).toString("utf8")));
    stream.end(JSON.stringify(collected));
  });
}

function observe(file) {
  const ran = spawnSync(process.execPath, [file], { cwd: dirname(file), encoding: "utf8", timeout: 20000 });
  return `exit=${ran.status}\n${ran.stdout}`;
}

const LICENSES = {
  browserify: "LICENSE", "browser-pack-6.1": "LICENSE", umd: "LICENSE", terser: "LICENSE", "uglify-js": "LICENSE",
  esbuild: "LICENSE.md", "@swc/core": "LICENSE",
};

function licenses() {
  const dir = join(OUT, "licenses");
  mkdirSync(dir, { recursive: true });
  for (const [name, file] of Object.entries(LICENSES)) {
    const label = name.replace("@", "").replace("/", "-").replace("-6.1", "");
    copyFileSync(join(HERE, "node_modules", name, file), join(dir, `LICENSE-${label}`));
  }
}

const manifest = [];

async function keep(cell, code, flags) {
  const dir = join(OUT, cell);
  mkdirSync(dir, { recursive: true });
  const file = join(dir, "index.js");
  writeFileSync(file, code);
  writeFileSync(join(OUT, "expected", `${cell}.stdout`), observe(file));
  manifest.push([cell, flags].join("\t"));
  console.log(cell);
}

async function variants(base, code, flags) {
  await keep(base, code, flags);
  for (const [name, minify] of Object.entries(MINIFIERS)) {
    await keep(`${base}+${name}`, await minify(code), `${flags} | ${MINIFIER_TOOLS[name]} ${version(MINIFIER_TOOLS[name])}`);
  }
}

async function main() {
  rmSync(OUT, { recursive: true, force: true });
  mkdirSync(join(OUT, "expected"), { recursive: true });
  const single = await rows(["index.js"]);
  const double = await rows(["index.js", "second.js"]);
  const entry = single.find((row) => row.entry).id;
  for (const name of PACKS) {
    const short = name.slice("browser-pack-".length);
    const flags = `browserify ${version("browserify")} rows | browser-pack ${version(name)}`;
    await variants(`bp${short}`, await pack(name, single, {}), flags);
    await keep(`bp${short}-two-entries`, await pack(name, double, {}), flags);
    if (STANDALONE.has(name)) {
      const code = await pack(name, single, { standalone: "matrix", standaloneModule: entry });
      await variants(`bp${short}-standalone`, code, `${flags} standalone`);
    }
  }
  await keep("bp6.1-exports", await pack("browser-pack-6.1", single, { hasExports: true }), `browserify ${version("browserify")} rows | browser-pack ${version("browser-pack-6.1")} hasExports`);
  writeFileSync(join(OUT, "MANIFEST"), `${manifest.join("\n")}\n`);
  licenses();
}

await main();
