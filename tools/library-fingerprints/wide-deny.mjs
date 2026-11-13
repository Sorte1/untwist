import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { gunzipSync } from "node:zlib";

const args = process.argv.slice(2);
const option = (flag, fallback) => (args.includes(flag) ? args[args.indexOf(flag) + 1] : fallback);
const XTASK = option("--xtask", null);
const TABLES = option("--tables", null);
const OUTPUTS = option("--outputs", null);
const BUNDLES = option("--bundles", null);
const DENY = option("--deny", null);
const COMMENT = /^\s*\/\/ (\S+) (\d[\d.,~–-]*): (.+?)\s*$/gm;
const HIT = /^ {2}line (\d+) \S+ \S+ outermost=\S+ (\S+) /;
const UNIT = /^ {2}unit \S+ ([0-9a-f]+) /;

function tableRows() {
  const byLabel = new Map();
  for (const file of readdirSync(TABLES).filter((name) => name.endsWith(".table.gz"))) {
    const text = gunzipSync(readFileSync(join(TABLES, file))).toString();
    const strings = text.slice(text.indexOf("#strings")).split("\n");
    const count = Number(strings[0].split("\t")[1]);
    const table = strings.slice(1, count + 1);
    const at = text.indexOf("#rows");
    for (const line of text.slice(text.indexOf("\n", at) + 1).split("\n")) {
      if (!line) continue;
      const cells = line.split("\t");
      const label = table[Number(cells[2])];
      if (!byLabel.has(label)) byLabel.set(label, new Set());
      byLabel.get(label).add(BigInt(`0x${cells[1]}`));
    }
  }
  return byLabel;
}

const tableArgs = readdirSync(TABLES).filter((name) => name.endsWith(".table.gz")).flatMap((name) => ["--table", join(TABLES, name)]);
const scan = (file, extra) => execFileSync(XTASK, ["library-scan", file, ...tableArgs, ...extra], { encoding: "utf8", maxBuffer: 1 << 30 });
const rows = tableRows();
const deny = new Map(existsSync(DENY) ? readFileSync(DENY, "utf8").split("\n").filter(Boolean).map((line) => {
  const [hash, label, ...why] = line.split("\t");
  return [`${hash}\t${label}`, why.join("\t")];
}) : []);
let added = 0;
for (const app of readdirSync(OUTPUTS).sort()) {
  const dir = join(OUTPUTS, app);
  for (const file of readdirSync(dir).filter((name) => name.endsWith(".js"))) {
    const bundler = file.split(".")[0];
    const truthFile = join(BUNDLES, app, `${bundler}.modules.tsv`);
    if (!existsSync(truthFile)) continue;
    const truth = new Set(readFileSync(truthFile, "utf8").split("\n").filter(Boolean).map((line) => line.split("\t")[0]));
    const text = readFileSync(join(dir, file), "utf8");
    const wrongLabels = new Set([...text.matchAll(COMMENT)].map((match) => match[1]).filter((label) => !truth.has(label)));
    if (!wrongLabels.size) continue;
    const lines = new Map();
    for (const line of scan(join(dir, file), ["--hits"]).split("\n")) {
      const match = HIT.exec(line);
      if (match && wrongLabels.has(match[2])) lines.set(match[1], match[2]);
    }
    for (const [line, label] of lines) {
      for (const unit of scan(join(dir, file), ["--line", line]).split("\n")) {
        const match = UNIT.exec(unit);
        if (!match) continue;
        const hash = BigInt(`0x${match[1]}`);
        if (!rows.get(label)?.has(hash)) continue;
        const key = `${hash.toString(16).padStart(16, "0")}\t${label}`;
        if (!deny.has(key)) {
          deny.set(key, `${app}/${file}:${line}`);
          added += 1;
        }
      }
    }
  }
}
const sorted = [...deny.entries()].sort(([a], [b]) => a.localeCompare(b)).map(([key, why]) => `${key}\t${why}`);
writeFileSync(DENY, `${sorted.join("\n")}\n`);
console.log(`${added} new deny rows, ${deny.size} total`);
