import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT = join(HERE, "..", "..", "fixtures", "unpack-rollup-assume");
const KEPT = ["EXPECT", "XFAIL"];
const ENTRY = join(HERE, "apps", "index.js");
const DOCUMENT = join(HERE, "document.mjs");
const require = createRequire(join(HERE, "package.json"));

const sha256 = (text) => createHash("sha256").update(text).digest("hex");
const version = (alias) => JSON.parse(readFileSync(join(HERE, "node_modules", alias, "package.json"), "utf8")).version;

const PAIRS = [
  ["rollup2", "cjs21", "es5"],
  ["rollup2", "cjs22", "es5"],
  ["rollup3", "cjs24", "es5"],
  ["rollup4", "cjs25", "es5"],
  ["rollup4", "cjs29", "es5"],
  ["rollup4", "cjs29", "es2015"],
];
const REQUIRES = ["default", "strict"];
const WRAPPERS = ["plain", "wrapped", "wrapped-strict"];

const WRAPPED_HEAD = "(function () {\n  var hostConfig;\n  var hostHook;\n";
const WRAPPED_TAIL = "})();\n";
const STRICT_HEAD = '(function () {\n  "use strict";\n  var hostConfig;\n  var hostHook;\n';
const STRICT_TAIL = '  hostConfig = { mode: "late" };\n  hostHook = function (n) {\n    return "hooked " + n;\n  };\n})();\n';

function wrapped(code, wrapper) {
  switch (wrapper) {
    case "plain":
      return code;
    case "wrapped":
      return WRAPPED_HEAD + code + WRAPPED_TAIL;
    case "wrapped-strict":
      return STRICT_HEAD + code + STRICT_TAIL;
  }
  throw new Error(`unknown wrapper ${wrapper}`);
}

async function bundled({ rollup: rollupName, plugin, requires, generated }) {
  const { rollup } = require(rollupName);
  const commonjs = require(plugin);
  const nodeResolve = require("node-resolve");
  const options = requires === "strict" ? { strictRequires: true } : {};
  const bundle = await rollup({
    input: ENTRY,
    plugins: [(nodeResolve.nodeResolve ?? nodeResolve)(), (commonjs.default ?? commonjs)(options)],
    onwarn: () => {},
  });
  const outputOptions = generated === "es5" ? { format: "iife" } : { format: "iife", generatedCode: generated };
  const { output } = await bundle.generate(outputOptions);
  await bundle.close?.();
  return output[0].code;
}

async function minified(code) {
  const { minify } = require("terser");
  return (await minify(code, { module: false })).code + "\n";
}

function printed(dir) {
  return execFileSync(process.execPath, ["--no-warnings", "--import", pathToFileURL(DOCUMENT).href, "index.js"], {
    cwd: dir,
    encoding: "utf8",
  });
}

function cells() {
  return PAIRS.flatMap(([rollup, plugin, generated]) =>
    REQUIRES.flatMap((requires) =>
      [false, true].flatMap((minify) =>
        WRAPPERS.map((wrapper) => ({ rollup, plugin, generated, requires, minify, wrapper })),
      ),
    ),
  );
}

const name = (cell) =>
  `${cell.rollup}-${cell.plugin}${cell.generated === "es5" ? "" : `-${cell.generated}`}-${cell.requires}${cell.minify ? "-min" : ""}-${cell.wrapper}`;

async function main() {
  mkdirSync(OUT, { recursive: true });
  for (const entry of readdirSync(OUT)) {
    if (!KEPT.includes(entry)) rmSync(join(OUT, entry), { recursive: true, force: true });
  }
  mkdirSync(join(OUT, "expected"), { recursive: true });
  copyFileSync(DOCUMENT, join(OUT, "document.mjs"));
  const manifest = [];
  const seen = new Map();
  for (const cell of cells()) {
    const id = name(cell);
    let code = await bundled(cell);
    if (!code.includes("currentScript")) throw new Error(`${id}: rollup emitted no import.meta.url stand-in`);
    if (cell.minify) code = await minified(code);
    const text = wrapped(code, cell.wrapper);
    const digest = sha256(text);
    if (seen.has(digest)) {
      console.log(`${id}: same output as ${seen.get(digest)}, dropped`);
      continue;
    }
    seen.set(digest, id);
    const dir = join(OUT, id);
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "index.js"), text);
    const out = printed(dir);
    if (!out.includes("host-cjs later")) throw new Error(`${id} did not run to the end:\n${out}`);
    writeFileSync(join(OUT, "expected", `${id}.stdout`), out);
    const tool = `rollup@${version(cell.rollup)}`;
    const wrapper = cell.wrapper === "plain" ? "none" : `${cell.wrapper}(written by tools/rollup-assume/gen.mjs, not rollup)`;
    const flags = `format=iife generatedCode=${cell.generated} commonjs@${version(cell.plugin)} strictRequires=${cell.requires} minify=${cell.minify} wrapper=${wrapper}`;
    manifest.push([`${id}/index.js`, tool, "node+document.mjs", flags, digest].join("\t"));
    console.log(`${id}: ok`);
  }
  writeFileSync(join(OUT, "MANIFEST"), "# path\ttool\tloader\tflags\tsha256\n" + manifest.join("\n") + "\n");
  mkdirSync(join(OUT, "licenses"), { recursive: true });
  copyFileSync(join(HERE, "node_modules", "rollup4", "LICENSE.md"), join(OUT, "licenses", "LICENSE-rollup"));
  copyFileSync(join(HERE, "node_modules", "cjs29", "LICENSE"), join(OUT, "licenses", "LICENSE-plugin-commonjs"));
  copyFileSync(join(HERE, "node_modules", "ms", "license.md"), join(OUT, "licenses", "LICENSE-ms"));
  copyFileSync(join(HERE, "node_modules", "object-assign", "license"), join(OUT, "licenses", "LICENSE-object-assign"));
}

await main();
