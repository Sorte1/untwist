import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { webcrack } from "webcrack";

const HERE = dirname(fileURLToPath(import.meta.url));
const ALLOWED = join(resolve(HERE, "../.."), "fixtures", "obfuscator-io") + sep;
const [listFile] = process.argv.slice(2);
if (!listFile) {
  process.stderr.write("usage: run.mjs <list of input\\toutput lines>\n");
  process.exit(2);
}

const pairs = readFileSync(listFile, "utf8")
  .split("\n")
  .filter(Boolean)
  .map((line) => line.split("\t"));

for (const [input, output] of pairs) {
  const started = performance.now();
  const report = (fields) => process.stdout.write(`${JSON.stringify({ input, ms: Math.round(performance.now() - started), ...fields })}\n`);
  if (!resolve(input).startsWith(ALLOWED)) {
    report({ ok: false, error: "refused: the oracle only runs on the generated fixtures/obfuscator-io files" });
    continue;
  }
  try {
    const result = await webcrack(readFileSync(input, "utf8"));
    writeFileSync(output, result.code);
    report({ ok: true });
  } catch (error) {
    writeFileSync(output, "");
    report({ ok: false, error: String(error?.message ?? error).slice(0, 300) });
  }
}
