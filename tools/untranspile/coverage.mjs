import { spawn } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { availableParallelism } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { restoredParts, residue } from "./markers.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const matrix = join(here, "../../fixtures/untranspile/matrix");
const untwist = process.env.UNTWIST_BIN ?? "untwist";
const outFile = process.argv[2] ?? join(here, ".work/coverage.tsv");
const lanes = [
  ["default", ["--no-unpack", "--no-jsx"]],
  ["unsafe", ["--no-unpack", "--no-jsx", "--unsafe"]],
];

const rows = readFileSync(join(matrix, "MANIFEST"), "utf8")
  .trim()
  .split("\n")
  .slice(1)
  .map((line) => line.split("\t"))
  .map(([tool, version, options, feature, status, file, faithful]) => ({ tool, version, options, feature, status, file, faithful }));

function deobfuscate(file, flags) {
  return new Promise((resolve) => {
    const child = spawn(untwist, [...flags, join(matrix, file)], { stdio: ["ignore", "pipe", "ignore"] });
    let out = "";
    child.stdout.on("data", (d) => (out += d));
    child.on("close", (code) => resolve(code === 0 ? out : null));
  });
}

const runDir = join(here, ".work/coverage-run");
mkdirSync(runDir, { recursive: true });

function observe(code) {
  const file = join(runDir, `c${createHash("sha1").update(code).digest("hex").slice(0, 16)}.js`);
  writeFileSync(file, code);
  return new Promise((resolve) => {
    const child = spawn(process.execPath, [file], { cwd: runDir, stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    const timer = setTimeout(() => child.kill("SIGKILL"), 20000);
    child.stdout.on("data", (d) => (stdout += d));
    child.stderr.on("data", (d) => (stderr += d));
    child.on("close", (status) => {
      clearTimeout(timer);
      resolve(`${status}\n${stdout}\n${(stderr.match(/^\w*Error\b.*$/m) ?? [""])[0]}`);
    });
  });
}

function grade(feature, before, after) {
  if (after === null) return "error";
  const left = residue(after).total;
  const now = restoredParts(feature, after);
  const was = restoredParts(feature, before.code);
  if (left === 0 && now.matched === now.of) return "restored";
  if (left < before.total || now.matched > was.matched) return "partial";
  return "none";
}

const files = [...new Set(rows.filter((r) => r.status === "written").map((r) => r.file))];
const graded = new Map();
let next = 0;
async function worker() {
  while (next < files.length) {
    const file = files[next++];
    const row = rows.find((r) => r.file === file);
    const code = readFileSync(join(matrix, file), "utf8");
    const before = { ...residue(code), code };
    const ready = restoredParts(row.feature, code);
    const result = { beforeParts: ready.matched, parts: ready.of, before: before.total, needed: before.total > 0 || ready.matched < ready.of, parity: [] };
    const expected = await observe(code);
    for (const [lane, flags] of lanes) {
      const after = await deobfuscate(file, flags);
      result[lane] = grade(row.feature, before, after);
      result[`${lane}Left`] = after === null ? -1 : residue(after).total;
      result[`${lane}Groups`] = after === null ? "" : Object.keys(residue(after).groups).join(",");
      result[`${lane}Parts`] = after === null ? 0 : restoredParts(row.feature, after).matched;
      if (after !== null && (await observe(after)) !== expected) result.parity.push(lane);
    }
    graded.set(file, result);
  }
}
await Promise.all(Array.from({ length: availableParallelism() }, worker));

const lines = ["tool\tversion\toptions\tfeature\tstatus\tbefore\tdefault\tunsafe\tdefaultLeft\tunsafeLeft\tleftGroups\tneeded\tparity\tparts\tbeforeParts\tdefaultParts\tunsafeParts"];
for (const r of rows) {
  const g = r.status === "written" || r.status === "same" ? graded.get(r.file) : null;
  if (!g) {
    lines.push([r.tool, r.version, r.options, r.feature, r.status.split(":")[0], "", "", "", "", "", "", "", "", "", "", "", ""].join("\t"));
    continue;
  }
  lines.push([r.tool, r.version, r.options, r.feature, r.status, g.before, g.default, g.unsafe, g.defaultLeft, g.unsafeLeft, g.unsafeGroups, g.needed ? "needed" : "nothing", g.parity.length ? "MISMATCH:" + g.parity.join("+") : "ok", g.parts, g.beforeParts, g.defaultParts, g.unsafeParts].join("\t"));
}
writeFileSync(outFile, lines.join("\n") + "\n");

const tally = new Map();
for (const r of rows) {
  const g = graded.get(r.file);
  if (!g || !(r.status === "written" || r.status === "same")) continue;
  const key = `${r.tool}\t${r.feature}`;
  const t = tally.get(key) ?? { cells: 0, default: 0, unsafe: 0, partial: 0 };
  t.cells++;
  if (g.default === "restored") t.default++;
  if (g.unsafe === "restored") t.unsafe++;
  else if (g.unsafe === "partial") t.partial++;
  tally.set(key, t);
}
const mismatched = [...graded].filter(([, g]) => g.parity.length);
for (const [file, g] of mismatched) console.log(`parity mismatch ${file} ${g.parity.join(",")}`);
console.log("tool\tfeature\tcells\tdefault restored\tunsafe restored\tunsafe partial");
for (const [key, t] of [...tally].sort()) console.log(`${key}\t${t.cells}\t${t.default}\t${t.unsafe}\t${t.partial}`);
