import { readFileSync } from "node:fs";
import { builtinModules, createRequire } from "node:module";
import { basename, dirname, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import vm from "node:vm";

const LOADERS = ["node", "systemjs", "amd", "bun-cjs"];
const tools = createRequire(new URL("./bundlers/package.json", import.meta.url));

function usage(message) {
  console.error(`unpack-run: ${message}\nusage: node tools/unpack-run.mjs [--loader ${LOADERS.join("|")}] <entry.js>`);
  process.exit(2);
}

function parseArgs(argv) {
  let loader = "node";
  const files = [];
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === "--loader") loader = argv[++i];
    else files.push(argv[i]);
  }
  if (!LOADERS.includes(loader)) usage(`unknown loader ${loader}`);
  if (files.length !== 1) usage("one entry file");
  return { loader, file: resolve(files[0]) };
}

async function runSystem(file) {
  const { System } = tools("systemjs/dist/system-node.cjs");
  globalThis.System = System;
  tools("systemjs/dist/extras/named-register.js");
  const proto = Object.getPrototypeOf(System);
  const resolveId = proto.resolve;
  const instantiate = proto.instantiate;
  const builtins = new Set(builtinModules);
  proto.resolve = function (id, parent) {
    const bare = id.startsWith("node:") ? id.slice(5) : id;
    return builtins.has(bare) ? `node:${bare}` : resolveId.call(this, id, parent);
  };
  proto.instantiate = function (url, parent) {
    if (!url.startsWith("node:")) return instantiate.call(this, url, parent);
    const host = createRequire(file)(url);
    return [[], (_export) => ({ execute() { _export({ ...host, default: host }); } })];
  };
  await System.import(pathToFileURL(file).href);
}

function runAmd(file) {
  const requirejs = tools("requirejs");
  requirejs.config({ nodeRequire: createRequire(file), baseUrl: dirname(file) });
  return new Promise((done, fail) => requirejs([basename(file, ".js")], done, fail));
}

function runBunCjs(file) {
  const factory = vm.runInThisContext(readFileSync(file, "utf8"), { filename: file });
  if (typeof factory !== "function") throw new Error(`${file} is not a bun-cjs wrapper`);
  const module = { exports: {}, filename: file, id: file };
  factory.call(module.exports, module.exports, createRequire(file), module, file, dirname(file));
}

const { loader, file } = parseArgs(process.argv.slice(2));
if (loader === "systemjs") await runSystem(file);
else if (loader === "amd") await runAmd(file);
else if (loader === "bun-cjs") runBunCjs(file);
else await import(pathToFileURL(file).href);
