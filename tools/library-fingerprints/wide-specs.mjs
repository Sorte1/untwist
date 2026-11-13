import { writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { LIBS, ISOLATED } from "../libcorpus/libs.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const OWNED_ELSEWHERE = new Set(["regenerator-runtime", "node-libs-browser"]);
const IDS = { "@babel/polyfill": "babel-polyfill-7", "core-js": "core-js-2" };
const OPTIONS = {
  three: { keepConcatenations: true, include: String.raw`/^build\/three\.(module|min|module\.min)\.js$|^examples\/jsm\/(controls|loaders|postprocessing|shaders|math|utils|geometries|objects|lines|exporters|environments|curves|modifiers|interactive|csm|effects|animation|misc)\//`, exclude: String.raw`/^examples\/jsm\/libs\//` },
  "pixi.js": { keepConcatenations: true, include: String.raw`/^dist\/(browser\/)?pixi\.min\.js$/` },
  svelte: { include: String.raw`/^(internal|src\/internal|src\/runtime)\//`, exclude: String.raw`/(^|\/)(compiler|server)(\/|\.|$)/` },
  react: { include: String.raw`/^(cjs|umd)\/react(-jsx-runtime)?\.(production\.min|development|production)\.js$/` },
  "react-dom": { include: String.raw`/^cjs\/react-dom(-client)?\.(production\.min|development|production)\.js$/` },
  moment: { include: String.raw`/^(moment\.js|min\/moment(-with-locales)?\.min\.js|(dist\/)?locale\/[^/]+\.js)$/` },
  "date-fns": { exclude: String.raw`/^(esm\/)?(fp|locale\/(?!en-US\/))|\/fp\//` },
  vue: { include: String.raw`/^dist\/vue\.(runtime\.esm|runtime\.esm-bundler|runtime\.min|min|global\.prod|runtime\.global\.prod)\.js$/` },
  lodash: { exclude: String.raw`/^fp\//` },
  "lodash-es": { label: "lodash" },
  "@msgpack/msgpack": { exclude: String.raw`/^dist\.(es5\+umd|umd)\/msgpack\.js$/` },
  "babel-polyfill": { exclude: String.raw`/^dist\/polyfill\.js$/` },
  "@babel/polyfill": { exclude: String.raw`/^dist\/polyfill\.js$/` },
  rxjs: { exclude: String.raw`/^(src|_esm2015|dist\/esm)\//` },
  mobx: { include: String.raw`/^(lib|dist)\/[^/]+\.js$/` },
  "@reduxjs/toolkit": { include: String.raw`/^dist\/[^/]+\.(c|m)?js$/` },
  "regenerator-runtime": null,
};


const GROUPS = {
  polyfills: ["core-js", "es5-shim", "es6-shim", "whatwg-fetch", "promise-polyfill", "es6-promise", "babel-polyfill", "@babel/polyfill",
    "buffer", "process", "events", "util", "path-browserify", "stream-browserify", "readable-stream", "crypto-browserify", "bn.js", "elliptic",
    "inherits", "safe-buffer", "base64-js", "ieee754"],
  react: ["react", "react-dom", "preact"],
  frameworks: ["vue", "svelte", "solid-js", "zone.js", "jquery"],
  lodash: ["lodash", "lodash-es", "underscore", "ramda"],
  dates: ["date-fns", "moment", "dayjs"],
  state: ["rxjs", "mobx", "redux", "@reduxjs/toolkit", "zustand", "immer"],
  data: ["axios", "protobufjs", "msgpack-lite", "@msgpack/msgpack", "pako", "fflate", "qs", "uuid", "js-cookie", "eventemitter3"],
  game: ["cannon", "cannon-es", "gl-matrix", "howler", "stats.js", "@tweenjs/tween.js", "tween.js", "socket.io-client", "engine.io-client"],
  pixi: ["pixi.js"],
  three: ["three"],
};
const SPLITS = {
  "pixi.js": [["pixi-js-4-6", String.raw`/^[456]\./`], ["pixi-js", String.raw`/^[78]\./`]],
};
const groupOf = (name) => Object.entries(GROUPS).find(([, names]) => names.includes(name))?.[0];

export const specId = (name) => IDS[name] ?? name.replace(/^@/, "").replace(/[/.]/g, "-");

const names = new Set();
for (const group of [LIBS, ISOLATED]) for (const libs of Object.values(group)) for (const name of Object.keys(libs)) names.add(name);
for (const name of [...names].sort()) {
  if (OWNED_ELSEWHERE.has(name)) continue;
  const extra = OPTIONS[name] ?? {};
  const group = groupOf(name);
  if (!group) throw new Error(`no group for ${name}`);
  for (const [id, versions] of SPLITS[name] ?? [[specId(name), null]]) {
  const options = [
    `id: ${JSON.stringify(id)}`,
    `group: ${JSON.stringify(group)}`,
    versions ? `versions: ${versions}` : null,
    `name: ${JSON.stringify(name)}`,
    `include: ${extra.include ?? "ALL"}`,
    extra.exclude ? `exclude: ${extra.exclude}` : null,
    extra.label ? `label: ${JSON.stringify(extra.label)}` : null,
    extra.keepConcatenations ? "keepConcatenations: true" : null,
  ].filter(Boolean);
  const source = `import { ALL, wide } from "../wide.mjs";\n\nexport default wide({ ${options.join(", ")} });\n`;
  writeFileSync(join(HERE, "libraries", `${id}.mjs`), source);
  }
}
