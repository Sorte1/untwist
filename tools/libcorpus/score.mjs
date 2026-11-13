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
const COMMENT = /^\s*\/\/ (\S+) (\d[\d.,~–-]*): (.+?)\s*$/gm;

const minor = (version) => version.split(".").slice(0, 2).map(Number);
const before = (a, b) => a[0] < b[0] || (a[0] === b[0] && a[1] <= b[1]);
const inRange = (range, version) => range.split(",").some((part) => {
  const [low, high = low] = part.split(/[~–-]/).map((piece) => minor(piece));
  const v = minor(version);
  return before(low, v) && before(v, high);
});
const CONCATENATED = /^(dist|build|bundles|umd)\/|[.-]min\.(c|m)?js$/;
const LABELS = { "lodash-es": "lodash" };
const labelOf = (lib) => LABELS[lib] ?? lib;
const stem = (file) => file.replace(/\.(c|m)?js$/, "");

function untwist(input, lane, saved) {
  const flags = lane === "unsafe" ? ["--unsafe"] : [];
  try {
    const text = execFileSync(UNTWIST, [...flags, input], { encoding: "utf8", maxBuffer: 1 << 30, stdio: ["ignore", "pipe", "pipe"], timeout: 600_000 });
    writeFileSync(saved, text);
    return text;
  } catch (error) {
    return null;
  }
}

function score(truthFile, output) {
  const truth = readFileSync(truthFile, "utf8").split("\n").filter(Boolean).map((line) => line.split("\t"));
  const comments = [...output.matchAll(COMMENT)].map((match) => ({ label: match[1], range: match[2], modules: match[3].split(" | ").map(stem), module: match[3] }));
  const versions = new Map(truth.map(([lib, version]) => [labelOf(lib), version]));
  const wrong = [];
  const ok = [];
  for (const comment of comments) {
    const version = versions.get(comment.label);
    if (!version) wrong.push(`${comment.label}:${comment.module} ${comment.range} (library not in bundle)`);
    else if (!inRange(comment.range, version)) wrong.push(`${comment.label}:${comment.module} ${comment.range} (truth ${version})`);
    else ok.push(comment);
  }
  let named = 0;
  let exact = 0;
  for (const [lib, , file] of truth) {
    const mine = ok.filter((comment) => comment.label === labelOf(lib));
    if (mine.some((comment) => comment.modules.includes(stem(file)))) {
      named += 1;
      exact += 1;
    } else if (CONCATENATED.test(file) && mine.length) named += 1;
  }
  const truthStems = new Set(truth.map(([lib, , file]) => `${labelOf(lib)}\t${stem(file)}`));
  const elsewhere = ok.filter((comment) => !comment.modules.some((module) => truthStems.has(`${comment.label}\t${module}`))).length;
  return { modules: truth.length, named, exact, comments: comments.length, elsewhere, wrong };
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
        mkdirSync(join(OUT, "outputs", app.id), { recursive: true });
        const output = untwist(input, lane, join(OUT, "outputs", app.id, `${bundler}${suffix.replace(/\.js$/, "")}.${lane}.js`));
        if (output === null) {
          rows.push([app.id, app.lib, app.version, bundler, post, lane, "error", "", "", "", "", "", "", "", Date.now() - started].join("\t"));
          continue;
        }
        const result = score(truthFile, output);
        rows.push([app.id, app.lib, app.version, bundler, post, lane, "ok", result.modules, result.named, result.exact, result.comments, result.elsewhere, result.wrong.length, result.wrong.join(" | "), Date.now() - started].join("\t"));
      }
    }
  }
}
mkdirSync(join(OUT, "scores"), { recursive: true });
writeFileSync(join(OUT, "scores", `score${option("--tag", "")}-${shard}-of-${shards}.tsv`), `${rows.join("\n")}\n`);
console.log(`${rows.length} rows, cpus ${availableParallelism()}`);
