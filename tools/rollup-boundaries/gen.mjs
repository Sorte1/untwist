import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT = join(HERE, "..", "..", "fixtures", "unpack-rollup-boundaries");
const KEPT = ["EXPECT", "XFAIL", "SCORES"];
const require = createRequire(join(HERE, "package.json"));
const { TraceMap, eachMapping } = require("@jridgewell/trace-mapping");
const { parseAst } = require("rollup4/parseAst");

const sha256 = (text) => createHash("sha256").update(text).digest("hex");
const version = (alias) => JSON.parse(readFileSync(join(HERE, "node_modules", alias, "package.json"), "utf8")).version;

const APPS = {
  synthetic: { entry: "apps/synthetic/main.js", resolve: {} },
  three: { entry: "apps/three/main.js", resolve: {} },
  solid: { entry: "apps/solid/main.js", resolve: { browser: true, exportConditions: ["browser"] } },
  svelte: { entry: "apps/svelte/main.js", resolve: {}, svelte: true },
};
const ROLLUPS = ["rollup3", "rollup4"];
const WRAPPERS = ["plain", "wrapped", "wrapped-strict"];

const WRAPPED_HEAD = "(function () {\n  var hostConfig;\n  var hostHook;\n";
const WRAPPED_TAIL = "})();\n";
const STRICT_HEAD = '(function () {\n  "use strict";\n  var hostConfig;\n  var hostHook;\n';
const STRICT_TAIL = '  hostConfig = { mode: "late" };\n  hostHook = function (n) {\n    return "hooked " + n;\n  };\n})();\n';

function wrapped(code, wrapper) {
  switch (wrapper) {
    case "plain":
      return { text: code, offset: 0 };
    case "wrapped":
      return { text: WRAPPED_HEAD + code + WRAPPED_TAIL, offset: WRAPPED_HEAD.length };
    case "wrapped-strict":
      return { text: STRICT_HEAD + code + STRICT_TAIL, offset: STRICT_HEAD.length };
  }
  throw new Error(`unknown wrapper ${wrapper}`);
}

function plugins(app) {
  const nodeResolve = require("node-resolve");
  const resolve = (nodeResolve.nodeResolve ?? nodeResolve)(app.resolve);
  if (!app.svelte) return [resolve];
  const svelte = require("rollup-plugin-svelte");
  return [(svelte.default ?? svelte)({ compilerOptions: { generate: "server" }, emitCss: false }), resolve];
}

async function bundled(cell) {
  const app = APPS[cell.app];
  const { rollup } = require(cell.rollup);
  const bundle = await rollup({ input: join(HERE, app.entry), plugins: plugins(app), onwarn: () => {} });
  const { output } = await bundle.generate({ format: "iife", sourcemap: true });
  await bundle.close?.();
  return { code: output[0].code, map: output[0].map };
}

async function minified({ code, map }) {
  const { minify } = require("terser");
  const result = await minify(code, { module: false, sourceMap: { content: map, asObject: true } });
  return { code: result.code + "\n", map: result.map };
}

function lineStarts(text) {
  const starts = [0];
  for (let at = 0; at < text.length; at++) if (text.charCodeAt(at) === 10) starts.push(at + 1);
  return starts;
}

function units(statement) {
  if (statement.type === "VariableDeclaration" && statement.declarations.length > 1) return statement.declarations;
  if (statement.type === "ExpressionStatement" && statement.expression.type === "SequenceExpression") {
    return statement.expression.expressions;
  }
  return [statement];
}

function innerStatements(code) {
  const ast = parseAst(code);
  const [statement] = ast.body;
  let expression = statement.expression;
  while (expression.type === "UnaryExpression") expression = expression.argument;
  return expression.callee.body.body.filter((item) => !item.directive).flatMap(units);
}

function sourceOf(sources, source) {
  return sources[source] == null ? null : relative(HERE, join(HERE, "apps", sources[source])).replace(/^(\.\.\/)+/, "");
}

function byteOffsets(text) {
  const bytes = new Uint32Array(text.length + 1);
  for (let at = 0; at < text.length; at++) {
    const unit = text.charCodeAt(at);
    const width = unit < 0x80 ? 1 : unit < 0x800 ? 2 : unit >= 0xd800 && unit < 0xdc00 ? 4 : unit >= 0xdc00 && unit < 0xe000 ? 0 : 3;
    bytes[at + 1] = bytes[at] + width;
  }
  return bytes;
}

function truth({ code, map }, offset) {
  const statements = innerStatements(code);
  const bytes = byteOffsets(code);
  const starts = lineStarts(code);
  const traced = new TraceMap(map);
  const votes = statements.map(() => new Map());
  let index = 0;
  const positions = [];
  eachMapping(traced, (mapping) => {
    if (mapping.source == null) return;
    positions.push([starts[mapping.generatedLine - 1] + mapping.generatedColumn, mapping.source]);
  });
  positions.sort((a, b) => a[0] - b[0]);
  for (const [at, source] of positions) {
    while (index < statements.length && statements[index].end <= at) index++;
    if (index === statements.length) break;
    if (statements[index].start > at) continue;
    votes[index].set(source, (votes[index].get(source) ?? 0) + 1);
  }
  const rows = [];
  let previous = null;
  for (const [at, statement] of statements.entries()) {
    const ranked = [...votes[at]].sort((a, b) => b[1] - a[1]);
    const module = ranked.length ? ranked[0][0] : previous;
    previous = module ?? previous;
    const label = module == null ? "(unmapped)" : module.replace(/^.*node_modules\//, "").replace(/^(\.\.\/)*apps\//, "app:");
    rows.push(`${bytes[statement.start] + offset}\t${bytes[statement.end] + offset}\t${ranked.length > 1 ? "mixed" : "one"}\t${label}`);
  }
  return rows.join("\n") + "\n";
}

function printed(dir) {
  return execFileSync(process.execPath, ["--no-warnings", "index.js"], { cwd: dir, encoding: "utf8" });
}

function cells() {
  return Object.keys(APPS).flatMap((app) =>
    ROLLUPS.flatMap((rollup) => [false, true].flatMap((minify) => WRAPPERS.map((wrapper) => ({ app, rollup, minify, wrapper })))),
  );
}

const name = (cell) => `${cell.app}-${cell.rollup}${cell.minify ? "-min" : ""}-${cell.wrapper}`;

async function main() {
  mkdirSync(OUT, { recursive: true });
  for (const entry of readdirSync(OUT)) {
    if (!KEPT.includes(entry)) rmSync(join(OUT, entry), { recursive: true, force: true });
  }
  mkdirSync(join(OUT, "expected"), { recursive: true });
  mkdirSync(join(OUT, "truth"), { recursive: true });
  const manifest = [];
  const seen = new Map();
  for (const cell of cells()) {
    const id = name(cell);
    let output = await bundled(cell);
    if (cell.minify) output = await minified(output);
    const { text, offset } = wrapped(output.code, cell.wrapper);
    const digest = sha256(text);
    if (seen.has(digest)) {
      console.log(`${id}: same output as ${seen.get(digest)}, dropped`);
      continue;
    }
    seen.set(digest, id);
    const dir = join(OUT, id);
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "index.js"), text);
    writeFileSync(join(OUT, "truth", `${id}.tsv`), truth(output, offset));
    writeFileSync(join(OUT, "expected", `${id}.stdout`), printed(dir));
    const tool = `rollup@${version(cell.rollup)}`;
    const wrapper = cell.wrapper === "plain" ? "none" : `${cell.wrapper}(written by tools/rollup-boundaries/gen.mjs, not rollup)`;
    const flags = `app=${cell.app} format=iife minify=${cell.minify ? `terser@${version("terser")}` : "false"} wrapper=${wrapper}`;
    manifest.push([`${id}/index.js`, tool, "node", flags, digest].join("\t"));
    console.log(`${id}: ok`);
  }
  writeFileSync(join(OUT, "MANIFEST"), "# path\ttool\tloader\tflags\tsha256\n" + manifest.join("\n") + "\n");
  mkdirSync(join(OUT, "licenses"), { recursive: true });
  copyFileSync(join(HERE, "node_modules", "rollup4", "LICENSE.md"), join(OUT, "licenses", "LICENSE-rollup"));
  copyFileSync(join(HERE, "node_modules", "three", "LICENSE"), join(OUT, "licenses", "LICENSE-three"));
  copyFileSync(join(HERE, "node_modules", "solid-js", "LICENSE"), join(OUT, "licenses", "LICENSE-solid-js"));
  copyFileSync(join(HERE, "node_modules", "svelte", "LICENSE.md"), join(OUT, "licenses", "LICENSE-svelte"));
}

await main();
