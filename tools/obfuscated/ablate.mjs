import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "../..");
const MANIFEST = join(REPO, "fixtures/obfuscator-io/MANIFEST");
const require = createRequire(join(HERE, "package.json"));
const Obfuscator = require("javascript-obfuscator-5");

const [sourceFile, program, outDir, only] = process.argv.slice(2);
if (!sourceFile || !program || !outDir) {
  console.error("usage: node tools/obfuscated/ablate.mjs <modified-source.js> <program> <out-dir> [cell-substring]");
  console.error("re-obfuscates the modified source with every matrix cell's options and seed for <program> (e.g. mixed); node runs only the obfuscator, never a sample");
  process.exit(2);
}

const code = readFileSync(sourceFile, "utf8");
mkdirSync(outDir, { recursive: true });
let written = 0;
for (const line of readFileSync(MANIFEST, "utf8").split("\n")) {
  if (!line || line.startsWith("#")) continue;
  const [file, cell, source, seed, , , options] = line.split("\t");
  if (basename(source, ".js") !== program) continue;
  if (only && !cell.includes(only)) continue;
  const settings = { ...JSON.parse(options), seed: Number(seed) };
  const out = Obfuscator.obfuscate(code, settings).getObfuscatedCode();
  writeFileSync(join(outDir, file), out + "\n");
  written += 1;
}
console.log(`ablate: ${written} cell(s) of ${program} written to ${outDir}`);
