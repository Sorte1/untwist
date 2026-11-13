import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "../..");
const FIXTURES = join(REPO, "fixtures/webpack-variants");
const untwist = process.env.UNTWIST_BIN ?? join(REPO, "target/release/untwist");
const work = process.env.WORK ?? join(process.env.TMPDIR ?? "/tmp", "webpack-variants-check");
const only = process.argv[2];
const flags = (process.env.UNTWIST_FLAGS ?? "").split(" ").filter(Boolean);

const rows = readFileSync(join(FIXTURES, "MANIFEST"), "utf8").split("\n").filter((l) => l && !l.startsWith("#"));
let passed = 0;
for (const row of rows) {
  const [cell, , , , files] = row.split("\t");
  if (only && !cell.includes(only)) continue;
  const out = join(work, cell);
  rmSync(out, { recursive: true, force: true });
  mkdirSync(out, { recursive: true });
  const inputs = files.split(",").map((f) => join(FIXTURES, cell, f));
  const run = spawnSync(untwist, [...flags, "-v", "-f", "-o", out, ...inputs], { encoding: "utf8" });
  const notes = (run.stderr ?? "").split("\n").filter((l) => l.startsWith("note: unpack")).slice(0, 3);
  if (!existsSync(join(out, "bundle.json"))) {
    console.log(`${cell}\tNO BUNDLE\t${notes.join(" | ")}`);
    continue;
  }
  const modules = JSON.parse(readFileSync(join(out, "bundle.json"), "utf8")).modules.length;
  const node = spawnSync(process.execPath, [join(out, "index.js")], { cwd: out, encoding: "utf8", timeout: 20000 });
  const expected = readFileSync(join(FIXTURES, `${cell}.stdout`), "utf8");
  const ok = node.status === 0 && node.stdout === expected;
  passed += ok;
  console.log(`${cell}\t${ok ? "PASS" : "MISMATCH"}\t${modules} modules\t${ok ? "" : (node.stderr.split("\n").find((l) => /Error/.test(l)) ?? node.stdout.split("\n").slice(0, 2).join("/"))}\t${notes.join(" | ")}`);
}
console.log(`passed ${passed}`);
