import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { availableParallelism } from "node:os";
import { join } from "node:path";
import { apps } from "./apps.mjs";

const OUT = process.env.UNTWIST_LIBCORPUS_OUT ?? join(process.env.HOME, ".cache/untwist-libcorpus");
const args = process.argv.slice(2);
const option = (flag, fallback) => (args.includes(flag) ? args[args.indexOf(flag) + 1] : fallback);
const UNTWIST = option("--untwist", null);
const LANES = option("--lanes", "default").split(",");
const only = new RegExp(option("--only", "."));
const [shard, shards] = option("--shard", "0/1").split("/").map(Number);
const BUNDLERS = ["webpack", "rollup", "esbuild", "vite"];
const POST = { none: ".js", terser: ".min.js", obfuscator: ".obf.js" };
const COMMENT = /^\s*\/\/ (\S+) (\S+): (\S+?)(\.(c|m)?js)?\s*$/gm;

const cmp = (a, b) => {
  const x = a.split(".").map(Number);
  const y = b.split(".").map(Number);
  for (let i = 0; i < Math.max(x.length, y.length); i += 1) if ((x[i] ?? 0) !== (y[i] ?? 0)) return (x[i] ?? 0) - (y[i] ?? 0);
  return 0;
};
const inRange = (range, version) => range.split(",").some((part) => {
  const [low, high = low] = part.split("~");
  return cmp(low, version) <= 0 && cmp(version, high) <= 0;
});

function untwist(input, lane) {
  const flags = lane === "unsafe" ? ["--unsafe"] : [];
  try {
    return execFileSync(UNTWIST, [...flags, input], { encoding: "utf8", maxBuffer: 1 << 30, stdio: ["ignore", "pipe", "pipe"], timeout: 600_000 });
  } catch (error) {
    return null;
  }
}

function score(truthFile, output) {
  const truth = readFileSync(truthFile, "utf8").split("\n").filter(Boolean).map((line) => line.split("\t"));
  const truthKeys = new Map(truth.map(([lib, version, file]) => [`${lib}\t${file.replace(/\.(c|m)?js$/, "")}`, version]));
  const named = new Map();
  for (const match of output.matchAll(COMMENT)) named.set(`${match[1]}\t${match[3]}`, match[2]);
  let right = 0;
  const wrong = [];
  for (const [key, range] of named) {
    const version = truthKeys.get(key);
    if (version && inRange(range, version)) right += 1;
    else wrong.push(`${key.replace("\t", ":")} ${range}${version ? ` (truth ${version})` : " (not in bundle)"}`);
  }
  return { modules: truth.length, right, wrong };
}

const rows = [];
const work = apps().filter((app, index) => only.test(app.id) && index % shards === shard);
for (const app of work) {
  for (const bundler of BUNDLERS) {
    const truthFile = join(OUT, "bundles", app.id, `${bundler}.modules.tsv`);
    if (!existsSync(truthFile)) continue;
    for (const [post, suffix] of Object.entries(POST)) {
      const input = join(OUT, "bundles", app.id, `${bundler}${suffix}`);
      if (!existsSync(input)) continue;
      for (const lane of LANES) {
        const started = Date.now();
        const output = untwist(input, lane);
        if (output === null) {
          rows.push([app.id, app.lib, app.version, bundler, post, lane, "error", "", "", "", Date.now() - started].join("\t"));
          continue;
        }
        const result = score(truthFile, output);
        rows.push([app.id, app.lib, app.version, bundler, post, lane, "ok", result.modules, result.right, result.wrong.join(" | "), Date.now() - started].join("\t"));
      }
    }
  }
}
mkdirSync(join(OUT, "scores"), { recursive: true });
writeFileSync(join(OUT, "scores", `score-${shard}-of-${shards}.tsv`), `${rows.join("\n")}\n`);
console.log(`${rows.length} rows, cpus ${availableParallelism()}`);
