import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import vm from "node:vm";

function parseArgs(argv) {
  let loader = "node";
  const files = [];
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === "--loader") loader = argv[++i];
    else files.push(resolve(argv[i]));
  }
  if (!["node", "browser", "amd", "system"].includes(loader) || files.length === 0) {
    console.error("usage: node run.mjs [--loader node|browser] <entry.js>...");
    process.exit(2);
  }
  return { loader, files };
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
    setAttribute(name, value) { attributes[name] = String(value); },
    getAttribute(name) { return attributes[name] ?? null; },
    addEventListener(type, listener) { this[`on${type}`] = listener; },
    removeEventListener() {},
  };
}

function runBrowser(files) {
  const base = dirname(files[files.length - 1]);
  const g = globalThis;
  const scripts = [];
  const execute = (path, script) => {
    g.document.currentScript = script;
    vm.runInThisContext(readFileSync(path, "utf8"), { filename: path });
    g.document.currentScript = null;
  };
  const head = {
    appendChild(el) {
      if (el.tagName !== "SCRIPT" || !el.src) return el;
      scripts.push(el);
      setTimeout(() => {
        try {
          execute(scriptPath(base, el.src), el);
          el.onload?.({ type: "load", target: el });
        } catch (error) {
          console.error(error);
          el.onerror?.({ type: "error", target: el });
        }
      }, 0);
      return el;
    },
    removeChild(el) { return el; },
  };
  g.window = g;
  g.self = g;
  g.location = { href: pathToFileURL(base + "/index.html").href };
  g.document = {
    head,
    body: head,
    currentScript: null,
    createElement: (tag) => element(tag, head),
    getElementsByTagName: (tag) => (tag === "script" ? scripts : []),
    querySelector: () => null,
  };
  for (const file of files) {
    const script = element("script", head);
    script.src = pathToFileURL(file).href;
    scripts.push(script);
    execute(file, script);
  }
}

function runAmd(file) {
  globalThis.require = createRequire(file);
  globalThis.define = (deps, factory) => {
    const run = typeof deps === "function" ? deps : factory;
    run();
  };
  globalThis.define.amd = {};
  vm.runInThisContext(readFileSync(file, "utf8"), { filename: file });
}

function runSystem(file) {
  globalThis.require = createRequire(file);
  let declared;
  globalThis.System = { register: (deps, declare) => { declared = declare; } };
  vm.runInThisContext(readFileSync(file, "utf8"), { filename: file });
  const module = declared(() => {}, { id: file, import: (id) => import(id) });
  module.execute();
}

const { loader, files } = parseArgs(process.argv.slice(2));
if (loader === "browser") runBrowser(files);
else if (loader === "amd") runAmd(files[0]);
else if (loader === "system") runSystem(files[0]);
else for (const file of files) await import(pathToFileURL(file).href);
