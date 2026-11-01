import { createRequire } from "node:module";
import { readFileSync, readdirSync, writeFileSync, mkdirSync } from "node:fs";
import { join, dirname, basename } from "node:path";
import { fileURLToPath } from "node:url";
import { homedir } from "node:os";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "..", "..");
const scratch = join(homedir(), ".cache/untwist/untranspile-tools");
const toolsDir = process.env.UNTRANSPILE_TOOLS ?? join(scratch, "tools");
const classicDir = process.env.UNTRANSPILE_CLASSIC ?? join(scratch, "tools-classic");
const pnpmStore = process.env.UNTRANSPILE_PNPM ?? join(process.env.UNTWIST_REFERENCE_DIR ?? join(repo, "reference"), "node_modules/.pnpm");
const outRoot = join(repo, "fixtures/untranspile");
const srcDir = join(here, "src");

const fromTools = createRequire(join(toolsDir, "package.json"));
const fromClassic = createRequire(join(classicDir, "package.json"));
const ts = fromTools("typescript");
const swc = fromTools("@swc/core");
const babel = fromTools("@babel/core");
const babelClassic = fromClassic("@babel/core");
const babelParser = fromTools("@babel/parser");
const babelGenerator = fromTools("@babel/generator").default;
const esbuilds = ["0.25.12", "0.27.7", "0.28.1"].map((version) => ({
  version,
  api: createRequire(join(pnpmStore, `esbuild@${version}/node_modules/esbuild/package.json`))("esbuild"),
}));
const minifier = esbuilds[esbuilds.length - 1].api;

function versionOf(require, name) {
  return require(`${name}/package.json`).version;
}

const versions = {
  node: process.version,
  typescript: ts.version,
  "@swc/core": versionOf(fromTools, "@swc/core"),
  "@babel/core": versionOf(fromTools, "@babel/core"),
  "@babel/preset-env": versionOf(fromTools, "@babel/preset-env"),
  "@babel/helpers": versionOf(fromTools, "@babel/helpers"),
  "@babel/core (classic)": versionOf(fromClassic, "@babel/core"),
  "@babel/preset-env (classic)": versionOf(fromClassic, "@babel/preset-env"),
  "@babel/helpers (classic)": versionOf(fromClassic, "@babel/helpers"),
  esbuild: esbuilds.map((e) => e.api.version),
  "esbuild (minifier)": minifier.version,
};

function tsc(code, target, extra = {}) {
  return ts.transpileModule(code, {
    compilerOptions: { target, module: ts.ModuleKind.None, removeComments: false, ...extra },
    reportDiagnostics: false,
  }).outputText;
}

function swcc(code, target) {
  return swc.transformSync(code, {
    jsc: { target, parser: { syntax: "ecmascript" }, externalHelpers: false },
    isModule: false,
    configFile: false,
    swcrc: false,
  }).code;
}

function babelc(core, code) {
  return core.transformSync(code, {
    presets: [[core === babel ? fromTools.resolve("@babel/preset-env") : fromClassic.resolve("@babel/preset-env"), { targets: { ie: 11 } }]],
    sourceType: "script",
    configFile: false,
    babelrc: false,
    compact: false,
  }).code;
}

function esbuildc(api, code, target) {
  return api.transformSync(code, { target, format: undefined, charset: "utf8" }).code;
}

function minify(code) {
  return minifier.transformSync(code, { minify: true, target: "esnext", charset: "utf8" }).code;
}

const variants = [
  ["ts", "es5", (c) => tsc(c, ts.ScriptTarget.ES5)],
  ["ts", "es5-iter", (c) => tsc(c, ts.ScriptTarget.ES5, { downlevelIteration: true })],
  ["ts", "es2015", (c) => tsc(c, ts.ScriptTarget.ES2015)],
  ["ts", "es2017", (c) => tsc(c, ts.ScriptTarget.ES2017)],
  ["swc", "es5", (c) => swcc(c, "es5")],
  ["swc", "es2015", (c) => swcc(c, "es2015")],
  ["babel", "7.29", (c) => babelc(babel, c)],
  ["babel", "classic", (c) => babelc(babelClassic, c)],
  ...esbuilds.map(({ version, api }) => ["esbuild", `${version}-es2015`, (c) => esbuildc(api, c, "es2015")]),
  ["esbuild", "0.28.1-es2017", (c) => esbuildc(esbuilds[2].api, c, "es2017")],
  ["esbuild", "0.28.1-es2019", (c) => esbuildc(esbuilds[2].api, c, "es2019")],
];
const minified = new Set(["ts/es5", "ts/es5-iter", "ts/es2015", "swc/es5", "babel/7.29", "babel/classic", "esbuild/0.28.1-es2015"]);

const written = [];
const refused = [];

function emit(dir, name, code) {
  mkdirSync(join(outRoot, dir), { recursive: true });
  const path = join(outRoot, dir, name);
  writeFileSync(path, code.endsWith("\n") ? code : code + "\n");
  written.push(`${dir}/${name}`);
}

for (const file of readdirSync(srcDir).filter((f) => f.endsWith(".js")).sort()) {
  const source = readFileSync(join(srcDir, file), "utf8");
  const stem = basename(file, ".js");
  for (const [tool, variant, compile] of variants) {
    let code;
    try {
      code = compile(source);
    } catch (error) {
      refused.push(`${tool}/${stem}.${variant}: ${String(error.message).split("\n")[0]}`);
      continue;
    }
    emit(tool, `${stem}.${variant}.js`, code);
    if (minified.has(`${tool}/${variant}`)) emit(tool, `${stem}.${variant}.min.js`, minify(code));
  }
}

function topLevel(code, keep) {
  const ast = babelParser.parse(code, { sourceType: "script" });
  return ast.program.body
    .filter(keep)
    .map((node) => babelGenerator(node).code)
    .join("\n");
}

function tslibPrelude(code) {
  return topLevel(code, (node) => node.type === "VariableDeclaration" && node.declarations.every((d) => d.id.name?.startsWith("__")));
}

function babelPrelude(code) {
  return topLevel(code, (node) => node.type === "FunctionDeclaration" && !sampleNames.has(node.id.name));
}

const sampleNames = new Set(["go", "s", "n", "Bar0", "Foo"]);
const sampleNeeds = `async function go(u) { return (await fetch(u)).json(); }
class Bar0 {} class Foo extends Bar0 { constructor(v) { super(v); this.v = v; } get val() { return this.v; } }
function s(a, b) { return [...a, ...b]; }
var g = { ...a, b: 1 }; var [x, y] = arr; async function n() { return [...await fetch(u), 1]; }`;

const tsSample = `function go(u) {
    return __awaiter(this, void 0, void 0, function () { var r; return __generator(this, function (_a) {
        switch (_a.label) {
            case 0: return [4 /*yield*/, fetch(u)];
            case 1: r = _a.sent(); return [2 /*return*/, r.json()];
        }
    }); });
}
var Foo = /** @class */ (function (_super) {
    __extends(Foo, _super);
    function Foo(v) { var _this = _super.call(this, v) || this; _this.v = v; return _this; }
    return Foo;
}(Bar));
function s(a, b) { return __spreadArray(__spreadArray([], a, true), b, true); }`;

const babelSample = `var Foo = /*#__PURE__*/function (_Bar) {
  function Foo(v) { var _this; _classCallCheck(this, Foo); _this = _callSuper(this, Foo, [v]); _this.v = v; return _this; }
  _inherits(Foo, _Bar);
  return _createClass(Foo, [{ key: "val", get: function get() { return this.v; } }]);
}(Bar);
var g = _objectSpread(_objectSpread({}, a), {}, { b: 1 });
var _arr = arr, _arr2 = _slicedToArray(_arr, 2), x = _arr2[0], y = _arr2[1];`;

const regeneratorSample = `_go = _asyncToGenerator(/*#__PURE__*/_regenerator().m(function _callee() {
  var r; return _regenerator().w(function (_context) {
    while (1) switch (_context.n) {
      case 0: _context.n = 1; return fetch(u);
      case 1: r = _context.v; return _context.a(2, [].concat(_toConsumableArray(r), [1]));
    }}, _callee); }));`;

const esbuildSampleTail = `function go(){return l(this,null,function*(){return(yield fetch(u)).json()})}`;

function esbuildAsyncHelper() {
  const out = minify(esbuildc(esbuilds[2].api, "async function go(){return (await fetch(u)).json()}", "es2015"));
  const match = /var __async=([\s\S]*?);function go/.exec(out);
  if (!match) throw new Error("esbuild __async helper not found");
  return `var l=${match[1]};`;
}

const tsStubs = `function Bar(v) { this.b = v; }
function fetch(u) { return Promise.resolve({ json: function () { return "json:" + u; } }); }`;
const tsTail = `go("x").then(function (v) { console.log(v); });
var foo = new Foo(3);
console.log(foo.v, foo.b, foo instanceof Bar, s([1, 2], [3]).join(","));`;
emit("samples", "ts-es5.js", [tslibPrelude(tsc(sampleNeeds, ts.ScriptTarget.ES5)), tsStubs, tsSample, tsTail].join("\n"));

const babelStubs = `function Bar(v) { this.b = v; }
var a = { p: 1, q: 2 };
var arr = ["first", "second", "third"];`;
const babelTail = `var foo = new Foo(4);
console.log(foo.val, foo.b, foo instanceof Bar, JSON.stringify(g), x, y);`;
emit("samples", "babel-ie11.js", [babelPrelude(babelc(babel, sampleNeeds)), babelStubs, babelSample, babelTail].join("\n"));

const regeneratorStubs = `var _go;
var u = "x";
function fetch(v) { return Promise.resolve([v, v]); }`;
const regeneratorTail = `_go().then(function (v) { console.log(v.join(",")); });`;
emit("samples", "babel-regenerator.js", [babelPrelude(babelc(babel, sampleNeeds)), regeneratorStubs, regeneratorSample, regeneratorTail].join("\n"));

const esbuildStubs = `var u = "x";
function fetch(v) { return Promise.resolve({ json() { return "json:" + v; } }); }`;
emit("samples", "esbuild-async.js", [esbuildStubs, esbuildAsyncHelper(), esbuildSampleTail, `go().then((v) => console.log(v));`].join("\n"));

writeFileSync(join(outRoot, "versions.json"), JSON.stringify({ versions, refused, files: written.length }, null, 2) + "\n");
console.log(`${written.length} fixtures, ${refused.length} refused`);
for (const line of refused) console.log(`refused ${line}`);
