import { readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

const [referenceEntry, listFile, mangleFlag = "mangle"] = process.argv.slice(2);
const mangle = mangleFlag !== "no-mangle";
if (!referenceEntry || !listFile) {
  process.stderr.write("usage: run-reference.mjs <reference dist/index.js> <list of input\\toutput lines> [mangle|no-mangle]\n");
  process.exit(2);
}

const exported = await import(pathToFileURL(referenceEntry).href);
const deobfuscate = Object.values(exported).find((value) => typeof value === "function");
if (!deobfuscate) {
  process.stderr.write(`${referenceEntry} exports no function\n`);
  process.exit(2);
}
const pairs = readFileSync(listFile, "utf8")
  .split("\n")
  .filter(Boolean)
  .map((line) => line.split("\t"));

for (const [input, output] of pairs) {
  const started = performance.now();
  try {
    const result = await deobfuscate(readFileSync(input, "utf8"), { mangle });
    writeFileSync(output, result.code);
    process.stdout.write(`${JSON.stringify({ input, ok: true, ms: Math.round(performance.now() - started) })}\n`);
  } catch (error) {
    writeFileSync(output, "");
    process.stdout.write(`${JSON.stringify({ input, ok: false, ms: Math.round(performance.now() - started), error: String(error?.message ?? error).slice(0, 300) })}\n`);
  }
}
