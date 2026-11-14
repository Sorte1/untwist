import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const MODULES = join(dirname(fileURLToPath(import.meta.url)), "..", "node_modules");

const versions = readdirSync(MODULES)
  .filter((dir) => /^core-js-3\.\d+$/.test(dir))
  .sort((a, b) => Number(a.split(".")[1]) - Number(b.split(".")[1]))
  .map((dir) => ({ dir, version: JSON.parse(readFileSync(join(MODULES, dir, "package.json"), "utf8")).version }));

export default {
  id: "core-js",
  label: "core-js",
  versions,
  entry(dir) {
    return `require("${dir}");\n`;
  },
  module(rel) {
    const match = /^(internals|modules)\/(.+)\.js$/.exec(rel);
    return match ? `${match[1]}/${match[2]}` : null;
  },
  minifiers: ["none", "terser", "terser-mangle", "esbuild", "swc"],
  obfuscate: "3.39.0,latest",
  group: "corejs",
};
