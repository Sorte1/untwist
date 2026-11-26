import { transformSync } from "@babel/core";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
const require = createRequire(import.meta.url);
export function compiled(source, filename) {
  return transformSync(source, { filename, babelrc: false, configFile: false, presets: [[require.resolve("babel-preset-solid"), { generate: "dom" }]] }).code;
}
if (process.argv[1] === fileURLToPath(import.meta.url) && process.argv[2]) process.stdout.write(compiled(readFileSync(process.argv[2], "utf8"), process.argv[2]));
