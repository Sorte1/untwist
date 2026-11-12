import { existsSync, readFileSync } from "node:fs";
import { builtinModules, createRequire } from "node:module";
import { basename, dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import vm from "node:vm";

const LOADERS = ["node", "browser", "systemjs", "amd"];
const tools = createRequire(new URL("./package.json", import.meta.url));

function usage(message) {
  console.error(`run: ${message}\nusage: node tools/bundle-matrix/run.mjs [--loader ${LOADERS.join("|")}] <entry.js>`);
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
  if (files.length !== 1 && !(loader === "browser" && files.length > 1)) usage("one entry file (the browser loader takes several, run in order)");
  return { loader, files: files.map((f) => resolve(f)) };
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

function scriptPath(base, src) {
  if (src.startsWith("file:")) return fileURLToPath(src);
  return resolve(base, src.replace(/^\.?\//, ""));
}

function element(tag, head) {
  const attributes = {};
  return {
    tagName: tag.toUpperCase(),
    parentNode: head,
    setAttribute(name, value) { attributes[name] = String(value); this[name] = String(value); },
    getAttribute(name) { return attributes[name] ?? null; },
    addEventListener(type, listener) { this[`on${type}`] = listener; },
    removeEventListener() {},
  };
}

function runBrowser(files) {
  const file = files[files.length - 1];
  const base = dirname(file);
  const g = globalThis;
  const head = {
    appendChild(el) {
      if (el.tagName !== "SCRIPT" || !el.src) return el;
      setTimeout(() => {
        try {
          const path = scriptPath(base, el.src);
          vm.runInThisContext(readFileSync(path, "utf8"), { filename: path });
          el.onload?.({ type: "load", target: el });
        } catch (error) {
          if (existsSync(scriptPath(base, el.src))) throw error;
          el.onerror?.({ type: "error", target: el });
        }
      });
      return el;
    },
    insertBefore(el) { return this.appendChild(el); },
    removeChild(el) { return el; },
  };
  const href = pathToFileURL(file).href;
  g.window = g;
  g.self = g;
  g.location = { href, protocol: "file:", origin: "null" };
  g.document = {
    head, body: head, baseURI: href,
    currentScript: { tagName: "SCRIPT", src: href, getAttribute: (n) => (n === "src" ? href : null) },
    createElement: (tag) => element(tag, head),
    getElementsByTagName: () => [],
    querySelector: () => null,
    querySelectorAll: () => [],
    addEventListener() {},
  };
  for (const script of files) vm.runInThisContext(readFileSync(script, "utf8"), { filename: script });
}

const { loader, files } = parseArgs(process.argv.slice(2));
const file = files[0];
if (loader === "systemjs") await runSystem(file);
else if (loader === "amd") await runAmd(file);
else if (loader === "browser") runBrowser(files);
else await import(pathToFileURL(file).href);
