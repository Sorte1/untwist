import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "../..");
const OUT = join(ROOT, "fixtures/obfuscators");
const SRC = join(HERE, "src");
const require = createRequire(join(HERE, "package.json"));
const version = (name) => require(`${name}/package.json`).version;
const source = (name) => readFileSync(join(SRC, name), "utf8");
const KRUNKER_ALPHABET = ["i", "í", "ì", "î", "ï"];
const WITNESSES = [
  ["encoders/", "\"packed\""],
  ["loader/", "\"packed\""],
  ["online-style/", "\"FizzBuzz\""],
  ["krunker-like/", "\"FizzBuzz\""],
  ["js-confuser/", "\"FizzBuzz\""],
  ["webpack-eval/", '!eval("'],
];

function witness(path) {
  return WITNESSES.find(([prefix]) => path.startsWith(prefix))?.[1] ?? "-";
}

function closure(level, languageOut) {
  const jar = join(HERE, "node_modules/google-closure-compiler-java/compiler.jar");
  const args = ["-jar", jar, "--compilation_level", level, "--language_out", languageOut, "--js", join(SRC, "program.js"), "--warning_level", "QUIET"];
  return execFileSync("java", args, { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] });
}

async function terser(options) {
  const { minify } = require("terser");
  return (await minify(source("program.js"), options)).code + "\n";
}

function uglify(options) {
  const result = require("uglify-js").minify(source("program.js"), options);
  if (result.error) throw result.error;
  return result.code + "\n";
}

function webpack(devtool) {
  const webpackLib = require("webpack");
  const out = mkdtempSync(join(tmpdir(), "untwist-obf-webpack-"));
  const config = {
    mode: "development",
    devtool,
    target: "node",
    context: join(SRC, "webpack"),
    entry: "./index.js",
    output: { path: out, filename: "main.js" },
  };
  return new Promise((resolve, reject) => {
    webpackLib(config, (error, stats) => {
      if (error || stats.hasErrors()) return reject(error ?? new Error(stats.toString("errors-only")));
      const text = readFileSync(join(out, "main.js"), "utf8").split(join(SRC, "webpack")).join(".");
      rmSync(out, { recursive: true, force: true });
      resolve(text);
    });
  });
}

function jsfuck() {
  return require("jsfuck").JSFuck.encode(source("tiny.js"), true) + "\n";
}

function aaencode() {
  return require("mangle-js-webpack-plugin/util/aaencode.js")(source("tiny.js")).getObfuscatedCode() + "\n";
}

function jjencode(prefix) {
  return require("mangle-js-webpack-plugin/util/jjencode.js")(source("tiny.js"), { prefix }).getObfuscatedCode() + "\n";
}

async function confuser(pkg, options) {
  const result = await require(pkg).obfuscate(source("program.js"), { target: "node", ...options });
  return (typeof result === "string" ? result : result.code) + "\n";
}

async function virtualizer(passes) {
  const { transpile } = require("js-virtualizer");
  const out = mkdtempSync(join(tmpdir(), "untwist-obf-jsvm-"));
  const result = await transpile(source("virtualize.js"), {
    fileName: "virtualize.js",
    writeOutput: false,
    vmOutputPath: join(out, "vm.js"),
    transpiledOutputPath: join(out, "out.js"),
    passes,
  });
  rmSync(out, { recursive: true, force: true });
  const shim = `var require = (function (load) {\n  var vm;\n  return function (id) {\n    if (!/vm\\.js$/.test(id)) return load(id);\n    if (!vm) {\n      vm = { exports: {} };\n      (function (module, exports, require) {\n${result.vm}\n      })(vm, vm.exports, load);\n    }\n    return vm.exports;\n  };\n})(require);\n`;
  const transpiled = shim + result.transpiled;
  return transpiled + "\n";
}

function krunkerDictionary() {
  const words = [];
  const grow = (prefix) => {
    if (prefix.length === 7) return words.push(prefix);
    for (const letter of KRUNKER_ALPHABET) grow(prefix + letter);
  };
  grow("i");
  return words;
}

function krunkerLike(pkg) {
  const Obfuscator = require(pkg);
  const options = {
    compact: true,
    seed: 1007,
    target: "browser",
    identifierNamesGenerator: "dictionary",
    identifiersDictionary: krunkerDictionary(),
    selfDefending: true,
    stringArray: true,
    stringArrayRotate: true,
    stringArrayShuffle: true,
    stringArrayIndexShift: true,
    stringArrayThreshold: 0.75,
    stringArrayEncoding: [],
    stringArrayWrappersCount: 1,
    stringArrayWrappersType: "variable",
    stringArrayWrappersChainedCalls: true,
  };
  return Obfuscator.obfuscate(source("program.js"), options).getObfuscatedCode() + "\n";
}

const b64 = (text) => Buffer.from(text, "utf8").toString("base64");

function loaders() {
  const tiny = source("tiny.js");
  const xorKey = 23;
  const xored = [...tiny].map((ch) => ch.charCodeAt(0) ^ xorKey).join(",");
  const uri = encodeURIComponent(tiny);
  return {
    "loader/function-atob.js": `Function(atob(${JSON.stringify(b64(tiny))}))();\n`,
    "loader/new-function-atob.js": `new Function(atob(${JSON.stringify(b64(tiny))}))();\n`,
    "loader/eval-decodeuri.js": `eval(decodeURIComponent(${JSON.stringify(uri)}));\n`,
    "loader/eval-xor.js": `eval([${xored}].map(function (c) { return String.fromCharCode(c ^ ${xorKey}); }).join(""));\n`,
    "loader/indirect-eval-atob.js": `(0, eval)(atob(${JSON.stringify(b64(tiny))}));\n`,
    "loader/constructor-chain.js": `[]["constructor"]["constructor"](atob(${JSON.stringify(b64(tiny))}))();\n`,
    "loader/eval-split-reverse.js": `eval(${JSON.stringify([...tiny].reverse().join(""))}.split("").reverse().join(""));\n`,
  };
}

function hexEscape(text) {
  return [...text].map((ch) => {
    const code = ch.charCodeAt(0);
    return code < 256 ? "\\x" + code.toString(16).padStart(2, "0").toUpperCase() : "\\u" + code.toString(16).padStart(4, "0");
  }).join("");
}

function hexArrayStyle(rotateAt) {
  const acorn = require("acorn");
  const walk = require("acorn-walk");
  const text = source("program.js");
  const tree = acorn.parse(text, { ecmaVersion: 2020 });
  const strings = [];
  const edits = [];
  const slot = (value) => {
    let at = strings.indexOf(value);
    if (at < 0) at = strings.push(value) - 1;
    return at;
  };
  walk.full(tree, (node) => {
    if (node.type === "Literal" && typeof node.value === "string") edits.push({ node, value: node.value, member: false });
    if (node.type === "MemberExpression" && !node.computed) edits.push({ node: node.property, value: node.property.name, member: true });
  });
  const keyed = new Set();
  walk.full(tree, (node) => {
    if (node.type === "Property" && !node.computed) keyed.add(node.key);
  });
  const name = "_0x3f1a";
  const shift = 3;
  const kept = edits.filter((edit) => !keyed.has(edit.node));
  kept.forEach((edit) => slot(edit.value));
  const count = strings.length;
  const stored = strings.slice(shift % count).concat(strings.slice(0, shift % count));
  let out = text;
  for (const edit of kept.sort((a, b) => b.node.start - a.node.start)) {
    const index = `${name}[${slot(edit.value)}]`;
    const replacement = edit.member ? `[${index}]` : index;
    const start = edit.member ? edit.node.start - 1 : edit.node.start;
    out = out.slice(0, start) + replacement + out.slice(edit.node.end);
  }
  const array = `var ${name} = [${stored.map((s) => `"${hexEscape(s)}"`).join(", ")}];\n`;
  const rotate = `(function (list, times) { while (times--) { list.unshift(list.pop()); } })(${name}, ${shift});\n`;
  if (rotateAt === "after-array") return array + rotate + out;
  const lines = out.split("\n");
  const at = lines.findIndex((line) => line.startsWith("function describe"));
  return array + lines.slice(0, at).join("\n") + "\n" + rotate + lines.slice(at).join("\n");
}

async function variants() {
  const program = source("program.js");
  return {
    "closure/simple.js": { tool: `google-closure-compiler@${version("google-closure-compiler")}`, options: "SIMPLE es5", text: closure("SIMPLE", "ECMASCRIPT5") },
    "closure/advanced-es5.js": { tool: `google-closure-compiler@${version("google-closure-compiler")}`, options: "ADVANCED es5", text: closure("ADVANCED", "ECMASCRIPT5") },
    "closure/advanced-es2015.js": { tool: `google-closure-compiler@${version("google-closure-compiler")}`, options: "ADVANCED es2015", text: closure("ADVANCED", "ECMASCRIPT_2015") },
    "mangle-props/terser-all.js": { tool: `terser@${version("terser")}`, options: "mangle.properties=true", text: await terser({ mangle: { properties: true } }) },
    "mangle-props/terser-toplevel-passes.js": { tool: `terser@${version("terser")}`, options: "toplevel compress.passes=3 unsafe mangle.properties=true", text: await terser({ toplevel: true, compress: { passes: 3, unsafe: true }, mangle: { properties: true } }) },
    "mangle-props/uglify-all.js": { tool: `uglify-js@${version("uglify-js")}`, options: "mangle.properties=true", text: uglify({ mangle: { properties: true } }) },
    "mangle-props/uglify-toplevel.js": { tool: `uglify-js@${version("uglify-js")}`, options: "toplevel mangle.properties=true", text: uglify({ toplevel: true, mangle: { properties: true } }) },
    "webpack-eval/eval.js": { tool: `webpack@${version("webpack")}`, options: "mode=development devtool=eval target=node", text: await webpack("eval") },
    "webpack-eval/eval-source-map.js": { tool: `webpack@${version("webpack")}`, options: "mode=development devtool=eval-source-map target=node", text: await webpack("eval-source-map") },
    "webpack-eval/eval-cheap-module-source-map.js": { tool: `webpack@${version("webpack")}`, options: "mode=development devtool=eval-cheap-module-source-map target=node", text: await webpack("eval-cheap-module-source-map") },
    "encoders/jsfuck-eval.js": { tool: `jsfuck@${version("jsfuck")}`, options: "wrapWithEval", text: jsfuck() },
    "encoders/aaencode.js": { tool: `mangle-js-webpack-plugin@${version("mangle-js-webpack-plugin")} util/aaencode`, options: "-", text: aaencode() },
    "encoders/jjencode-dollar.js": { tool: `mangle-js-webpack-plugin@${version("mangle-js-webpack-plugin")} util/jjencode`, options: "prefix=$", text: jjencode("$") },
    "encoders/jjencode-word.js": { tool: `mangle-js-webpack-plugin@${version("mangle-js-webpack-plugin")} util/jjencode`, options: "prefix=thinkzhan", text: jjencode("thinkzhan") },
    "js-confuser/v1-low.js": { tool: `js-confuser@${version("js-confuser-1")}`, options: "preset=low", text: await confuser("js-confuser-1", { preset: "low" }) },
    "js-confuser/v1-medium.js": { tool: `js-confuser@${version("js-confuser-1")}`, options: "preset=medium", text: await confuser("js-confuser-1", { preset: "medium" }) },
    "js-confuser/v1-high.js": { tool: `js-confuser@${version("js-confuser-1")}`, options: "preset=high", text: await confuser("js-confuser-1", { preset: "high" }) },
    "js-confuser/v2.0-low.js": { tool: `js-confuser@${version("js-confuser")}`, options: "preset=low", text: await confuser("js-confuser", { preset: "low" }) },
    "js-confuser/v2.0-medium.js": { tool: `js-confuser@${version("js-confuser")}`, options: "preset=medium", text: await confuser("js-confuser", { preset: "medium" }) },
    "js-confuser/v2.0-high.js": { tool: `js-confuser@${version("js-confuser")}`, options: "preset=high", text: await confuser("js-confuser", { preset: "high" }) },
    "js-confuser/v2.1-medium.js": { tool: `js-confuser@${version("js-confuser-2.1")}`, options: "preset=medium", text: await confuser("js-confuser-2.1", { preset: "medium" }) },
    "js-confuser/v2.1-strings-only.js": { tool: `js-confuser@${version("js-confuser-2.1")}`, options: "stringConcealing stringEncoding stringSplitting", text: await confuser("js-confuser-2.1", { stringConcealing: true, stringEncoding: true, stringSplitting: true }) },
    "vm/js-virtualizer-plain.js": { tool: `js-virtualizer@${version("js-virtualizer")}`, options: "passes=[], VM module inlined behind a require shim by gen.mjs", text: await virtualizer([]) },
    "vm/js-virtualizer-default.js": { tool: `js-virtualizer@${version("js-virtualizer")}`, options: "passes=RemoveUnused,ObfuscateVM,ObfuscateTranspiled, VM module inlined behind a require shim by gen.mjs", text: await virtualizer(["RemoveUnused", "ObfuscateVM", "ObfuscateTranspiled"]) },
    "krunker-like/obfuscator-4.js": { tool: `javascript-obfuscator@${version("javascript-obfuscator")}`, options: "dictionary i+6 of iíìîï, selfDefending, stringArray rotate shuffle indexShift threshold=0.75 encoding=none wrappers=1 variable chained, seed=1007", text: krunkerLike("javascript-obfuscator") },
    "online-style/hex-array-rotate-top.js": { tool: "gen.mjs", options: "hex-escaped string array, rotate IIFE after the array", text: hexArrayStyle("after-array") },
    "online-style/hex-array-rotate-middle.js": { tool: "gen.mjs", options: "hex-escaped string array, rotate IIFE between functions", text: hexArrayStyle("middle") },
    ...Object.fromEntries(Object.entries(loaders()).map(([path, text]) => [path, { tool: "gen.mjs", options: "loader form", text }])),
    "plain/program.js": { tool: "source", options: "-", text: program },
  };
}

function sha256(text) {
  return createHash("sha256").update(text).digest("hex");
}

function oldRows() {
  try {
    const rows = new Map();
    for (const line of readFileSync(join(OUT, "MANIFEST"), "utf8").split("\n")) {
      if (!line || line.startsWith("#")) continue;
      const fields = line.split("\t");
      rows.set(fields[0], fields);
    }
    return rows;
  } catch {
    return new Map();
  }
}

const rows = oldRows();
const made = await variants();
rmSync(OUT, { recursive: true, force: true });
const lines = [
  `# node ${process.version}; generated by tools/obfuscators/gen.mjs from tools/obfuscators/src; tool versions pinned in tools/obfuscators/package.json`,
  "# witness: text the output holds once the obfuscation is undone (!text: text it no longer holds); safe/unsafe: undone, kept or - (node parity only)",
  "# path\ttool\toptions\twitness\tsafe\tunsafe\tsha256",
];
for (const [path, { tool, options, text }] of Object.entries(made).sort(([a], [b]) => a.localeCompare(b))) {
  mkdirSync(dirname(join(OUT, path)), { recursive: true });
  writeFileSync(join(OUT, path), text);
  const old = rows.get(path);
  const mark = witness(path);
  const [safe, unsafe] = old && old[3] === mark ? [old[4], old[5]] : [mark === "-" ? "-" : "?", mark === "-" ? "-" : "?"];
  lines.push([path, tool, options, mark, safe, unsafe, sha256(text)].join("\t"));
}
writeFileSync(join(OUT, "MANIFEST"), lines.join("\n") + "\n");
console.log(`wrote ${Object.keys(made).length} fixtures to ${relative(ROOT, OUT)}`);
