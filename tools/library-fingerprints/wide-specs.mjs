import { writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { LIBS, ISOLATED } from "../libcorpus/libs.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const OWNED_ELSEWHERE = new Set(["regenerator-runtime", "node-libs-browser"]);
const IDS = { "@babel/polyfill": "babel-polyfill-7", "core-js": "core-js-2" };
const OPTIONS = {
  three: { include: String.raw`/^build\/three\.(module|min|module\.min)\.js$|^examples\/jsm\/(controls|loaders|postprocessing|shaders|math|utils|geometries|objects|lines|exporters|environments|curves|modifiers|interactive|csm|effects|animation|misc)\//`, exclude: String.raw`/^examples\/jsm\/libs\//` },
  "pixi.js": { include: String.raw`/^dist\/(browser\/)?pixi(\.min)?\.(m?js)$/` },
  svelte: { include: String.raw`/^(internal|src\/internal|src\/runtime)\//`, exclude: String.raw`/(^|\/)(compiler|server)(\/|\.|$)/` },
  react: { include: String.raw`/^(cjs|umd)\/react(-jsx-runtime)?\.(production\.min|development|production)\.js$/` },
  "react-dom": { include: String.raw`/^(cjs|umd)\/react-dom(-client|-server\.browser|-server-legacy\.browser)?\.(production\.min|development|production)\.js$/` },
  moment: { include: String.raw`/^(moment\.js|min\/moment(-with-locales)?\.min\.js|(dist\/)?locale\/[^/]+\.js)$/` },
  "date-fns": { exclude: String.raw`/^(esm\/)?(fp|locale\/(?!en-US\/))|\/fp\//` },
  vue: { include: String.raw`/^dist\/[^/]+\.js$/`, exclude: String.raw`/compiler|server|ssr/` },
  lodash: { exclude: String.raw`/^fp\//` },
  rxjs: { exclude: String.raw`/^(src|_esm2015|dist\/esm)\//` },
  mobx: { include: String.raw`/^(lib|dist)\/[^/]+\.js$/` },
  "@reduxjs/toolkit": { include: String.raw`/^dist\/[^/]+\.(c|m)?js$/` },
  "regenerator-runtime": null,
};


export const specId = (name) => IDS[name] ?? name.replace(/^@/, "").replace(/[/.]/g, "-");

const names = new Set();
for (const group of [LIBS, ISOLATED]) for (const libs of Object.values(group)) for (const name of Object.keys(libs)) names.add(name);
for (const name of [...names].sort()) {
  if (OWNED_ELSEWHERE.has(name)) continue;
  const id = specId(name);
  const extra = OPTIONS[name] ?? {};
  const options = [
    `id: ${JSON.stringify(id)}`,
    `name: ${JSON.stringify(name)}`,
    `include: ${extra.include ?? "ALL"}`,
    extra.exclude ? `exclude: ${extra.exclude}` : null,
  ].filter(Boolean);
  const source = `import { ALL, wide } from "../wide.mjs";\n\nexport default wide({ ${options.join(", ")} });\n`;
  writeFileSync(join(HERE, "libraries", `${id}.mjs`), source);
}
