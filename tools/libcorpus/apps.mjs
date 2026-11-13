import { entries } from "./libs.mjs";

const SIDE_EFFECT = new Set([
  "core-js", "es5-shim", "es6-shim", "whatwg-fetch", "babel-polyfill", "@babel/polyfill", "regenerator-runtime", "zone.js",
]);
const CORPUS_ONLY = new Set(["node-libs-browser"]);
const DEV_VARIANTS = new Set(["react", "react-dom", "vue", "mobx"]);
const PEERS = { "react-dom": [["react", "same-minor"]], zustand: [["react", "18.3.1"]] };

const SUBPATH = { svelte: (version) => (version.startsWith("5.") ? "svelte/internal/client" : "svelte/internal") };

const whole = (name, version) =>
  SUBPATH[name]
    ? `import * as lib from ${JSON.stringify(SUBPATH[name](version))};\nglobalThis.__lib = lib;\n`
    : SIDE_EFFECT.has(name)
    ? `import ${JSON.stringify(name)};\nglobalThis.__loaded = true;\n`
    : `import * as lib from ${JSON.stringify(name)};\nglobalThis.__lib = lib;\n`;

const SELECTIVE = [
  ["lodash", "4.17.21", "per-method", 'import debounce from "lodash/debounce";\nimport cloneDeep from "lodash/cloneDeep";\nimport get from "lodash/get";\nglobalThis.__lib = { debounce, cloneDeep, get };\n'],
  ["lodash-es", "4.17.21", "named", 'import { debounce, cloneDeep, get, merge } from "lodash-es";\nglobalThis.__lib = { debounce, cloneDeep, get, merge };\n'],
  ["core-js", "2.6.12", "es6-promise", 'import "core-js/es6/promise";\nimport "core-js/fn/array/from";\nimport "core-js/fn/object/assign";\nglobalThis.__loaded = true;\n'],
  ["three", "0.128.0", "named", 'import { WebGLRenderer, Scene, PerspectiveCamera, Mesh, BoxGeometry, MeshBasicMaterial } from "three";\nglobalThis.__lib = { WebGLRenderer, Scene, PerspectiveCamera, Mesh, BoxGeometry, MeshBasicMaterial };\n'],
  ["three", "0.128.0", "orbit-controls", 'import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";\nimport { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";\nglobalThis.__lib = { OrbitControls, GLTFLoader };\n'],
  ["three", "0.160.1", "orbit-controls", 'import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";\nimport { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";\nglobalThis.__lib = { OrbitControls, GLTFLoader };\n'],
  ["rxjs", "6.6.7", "operators", 'import { of } from "rxjs";\nimport { map, filter } from "rxjs/operators";\nglobalThis.__lib = { of, map, filter };\n'],
  ["date-fns", "2.30.0", "named", 'import { format, addDays, parseISO } from "date-fns";\nglobalThis.__lib = { format, addDays, parseISO };\n'],
  ["pixi.js", "7.4.3", "named", 'import { Application, Sprite, Graphics } from "pixi.js";\nglobalThis.__lib = { Application, Sprite, Graphics };\n'],
];

export function apps() {
  const out = [];
  const byKey = new Map(entries().map((entry) => [`${entry.name}@${entry.version}`, entry]));
  const aliasesFor = (entry) => {
    const map = { [entry.name]: entry.alias };
    for (const [peer, pinned] of PEERS[entry.name] ?? []) {
      const minor = entry.version.split(".").slice(0, 2).join(".");
      const match = pinned === "same-minor"
        ? [...byKey.values()].find((candidate) => candidate.name === peer && candidate.version.startsWith(`${minor}.`))
        : byKey.get(`${peer}@${pinned}`);
      if (match) map[peer] = match.alias;
    }
    return map;
  };
  for (const entry of entries()) {
    if (CORPUS_ONLY.has(entry.name)) continue;
    const base = { lib: entry.name, version: entry.version, root: entry.root, aliases: aliasesFor(entry) };
    out.push({ ...base, id: `${entry.slug}--whole`, env: "production", source: whole(entry.name, entry.version) });
    if (DEV_VARIANTS.has(entry.name)) out.push({ ...base, id: `${entry.slug}--whole-dev`, env: "development", source: whole(entry.name, entry.version) });
  }
  for (const [name, version, kind, source] of SELECTIVE) {
    const entry = byKey.get(`${name}@${version}`);
    out.push({ lib: name, version, root: entry.root, aliases: aliasesFor(entry), id: `${entry.slug}--${kind}`, env: "production", source });
  }
  out.push({ lib: "baseline", version: "0.0.0", root: ".", aliases: {}, id: "baseline--esm", env: "production", source: "export const value = 1;\nglobalThis.__lib = { value };\n" });
  out.push({ lib: "baseline", version: "0.0.0", root: ".", aliases: {}, id: "baseline--cjs", env: "production", source: "module.exports = { value: 1 };\nglobalThis.__lib = require(\"./index.js\");\n" });
  return out;
}
