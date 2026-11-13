import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "../..");
const FIXTURES = join(ROOT, "fixtures/obfuscators");
const MANIFEST = join(FIXTURES, "MANIFEST");
const UNTWIST = process.env.UNTWIST ?? join(ROOT, "target/release/untwist");
const TIMEOUT_MS = 20000;
const LANES = [["safe", []], ["unsafe", ["--unsafe"]]];

function run(file) {
  const result = spawnSync(process.execPath, [file], { encoding: "utf8", timeout: TIMEOUT_MS });
  return `${result.stdout}exit ${result.status}${result.signal ? " " + result.signal : ""}`;
}

function untwist(file, flags) {
  const result = spawnSync(UNTWIST, [file, ...flags], { encoding: "utf8", maxBuffer: 1 << 30, timeout: 600000 });
  return result.status === 0 ? result.stdout : null;
}

function undone(witness, code) {
  if (witness === "-") return "-";
  const held = witness.startsWith("!") ? !code.includes(witness.slice(1)) : code.includes(witness);
  return held ? "undone" : "kept";
}

const write = process.argv.includes("--write");
const only = process.argv.slice(2).filter((arg) => arg !== "--write");
const lines = readFileSync(MANIFEST, "utf8").split("\n");
const dir = mkdtempSync(join(tmpdir(), "untwist-obf-check-"));
let failed = 0;
const updated = lines.map((line) => {
  if (!line || line.startsWith("#")) return line;
  const fields = line.split("\t");
  const [rel, , , witness] = fields;
  if (only.length && !only.some((prefix) => rel.startsWith(prefix))) return line;
  const file = join(FIXTURES, rel);
  const before = run(file);
  const cells = [rel];
  LANES.forEach(([lane, flags], at) => {
    const code = untwist(file, flags);
    if (code === null) {
      failed += 1;
      cells.push(`${lane}: untwist failed`);
      return;
    }
    const outFile = join(dir, `${lane}.js`);
    writeFileSync(outFile, code);
    const after = run(outFile);
    const outcome = undone(witness, code);
    if (after !== before) failed += 1;
    if (outcome !== fields[4 + at]) cells.push(`${lane} MANIFEST ${fields[4 + at]}`);
    cells.push(`${lane}:${after === before ? "same" : "DIFF"}:${outcome}`);
    fields[4 + at] = outcome;
  });
  console.log(cells.join("\t"));
  return fields.join("\t");
});
rmSync(dir, { recursive: true, force: true });
if (write) writeFileSync(MANIFEST, updated.join("\n"));
if (failed) {
  console.error(`${failed} lane run(s) failed or behaved unlike node`);
  process.exit(1);
}
