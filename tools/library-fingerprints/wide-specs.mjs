import { writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { LIBS, ISOLATED } from "../libcorpus/libs.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const OWNED_ELSEWHERE = new Set(["regenerator-runtime", "node-libs-browser"]);
const IDS = { "@babel/polyfill": "babel-polyfill-7", "core-js": "core-js-2" };
const OPTIONS = {
  three: "exclude: /^src\\//",
  svelte: 'exclude: /(^|\\/)compiler[^/]*$|^compiler\\//',
};

export const specId = (name) => IDS[name] ?? name.replace(/^@/, "").replace(/[/.]/g, "-");

const names = new Set();
for (const group of [LIBS, ISOLATED]) for (const libs of Object.values(group)) for (const name of Object.keys(libs)) names.add(name);
for (const name of [...names].sort()) {
  if (OWNED_ELSEWHERE.has(name)) continue;
  const id = specId(name);
  const options = [`id: ${JSON.stringify(id)}`, `name: ${JSON.stringify(name)}`, "include: ALL", OPTIONS[name]].filter(Boolean);
  const source = `import { ALL, wide } from "../wide.mjs";\n\nexport default wide({ ${options.join(", ")} });\n`;
  writeFileSync(join(HERE, "libraries", `${id}.mjs`), source);
}
