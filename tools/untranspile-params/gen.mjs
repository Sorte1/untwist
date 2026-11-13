import { createRequire } from "node:module";
import { readFileSync, readdirSync, writeFileSync, mkdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const outRoot = join(here, "..", "..", "fixtures/untranspile/params");
const require = createRequire(join(here, "package.json"));
const babel = require("@babel/core");
const ts = require("typescript");
const swc = require("@swc/core");

const babelPreset = require.resolve("@babel/preset-env");

function babelc(code, loose) {
  return babel.transformSync(code, {
    presets: [[babelPreset, { targets: { ie: 11 }, loose }]],
    babelrc: false,
    configFile: false,
  }).code;
}

function tsc(code, target) {
  return ts.transpileModule(code, {
    compilerOptions: { target, module: ts.ModuleKind.None },
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

const variants = [
  ["babel-7.29", (c) => babelc(c, false)],
  ["babel-7.29-loose", (c) => babelc(c, true)],
  ["ts-5.9-es5", (c) => tsc(c, ts.ScriptTarget.ES5)],
  ["swc-1.16-es5", (c) => swcc(c, "es5")],
];

const versions = {
  "@babel/core": require("@babel/core/package.json").version,
  "@babel/preset-env": require("@babel/preset-env/package.json").version,
  typescript: ts.version,
  "@swc/core": require("@swc/core/package.json").version,
};

for (const file of readdirSync(join(here, "src")).filter((f) => f.endsWith(".js")).sort()) {
  const code = readFileSync(join(here, "src", file), "utf8");
  for (const [variant, compile] of variants) {
    const dir = join(outRoot, variant);
    mkdirSync(dir, { recursive: true });
    const out = compile(code);
    writeFileSync(join(dir, file), out.endsWith("\n") ? out : out + "\n");
  }
}
writeFileSync(join(outRoot, "versions.json"), JSON.stringify(versions, null, 2) + "\n");
