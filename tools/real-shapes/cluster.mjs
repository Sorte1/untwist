import { spawn } from "node:child_process";
import { mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const NAME = /^(game|loader)-([0-9a-f]+)-(.+)\.js$/;
const CAP = ["--user", "--scope", "-q", "--slice=untwist.slice", "-p", "MemoryMax=4G", "-p", "MemorySwapMax=0"];
const TIMEOUT_S = "600";
const BASE64 = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=";

function parseArgs(argv) {
  const options = { jobs: "2" };
  for (let i = 0; i < argv.length; i += 2) options[argv[i].replace(/^--/, "")] = argv[i + 1];
  if (!options.export || !options.untwist || !options.out) {
    throw new Error("usage: cluster.mjs --export <dir> --untwist <bin> --out <dir> [--jobs N]");
  }
  return { ...options, jobs: Number(options.jobs) };
}

function run(bin, args) {
  return new Promise((resolve) => {
    const child = spawn("systemd-run", [...CAP, "timeout", TIMEOUT_S, bin, ...args]);
    const out = [];
    const err = [];
    child.stdout.on("data", (chunk) => out.push(chunk));
    child.stderr.on("data", (chunk) => err.push(chunk));
    child.on("close", (status) => resolve({ status, stdout: Buffer.concat(out).toString("utf8"), stderr: Buffer.concat(err).toString("utf8") }));
  });
}

function bucket(n) {
  return n === 0 ? "0" : `1e${Math.floor(Math.log10(n))}`;
}

function identStyle(code) {
  const hex = (code.match(/\b_0x[0-9a-f]{4,6}\b/g) ?? []).length;
  const confusable = (code.match(/[A-Za-z_$]*[^\x00-\x7f][A-Za-z_$\u0080-￿]*\s*\(/g) ?? []).length;
  if (hex > confusable && hex > 100) return "hex";
  if (confusable > 100) return "confusable";
  return "plain";
}

function inputFeatures(code) {
  return {
    style: identStyle(code),
    base64: code.includes(BASE64) ? "b64" : "-",
    rc4: /0x100|\b256\b/.test(code) && code.includes(BASE64) ? "rc4" : "-",
    evalRequire: /eval\(\s*['"]require;?['"]\s*\)/.test(code) ? "eval-require" : "-",
    selfDefending: /\(\(\(\.\+\)\+\)\+\)\+\$|\(\(\(\.\+\)\+\)\+\)\+/.test(code) ? "self-defending" : "-",
    debug: /debugger/.test(code) ? "debugger" : "-",
    rotator: /\['push'\]\(\w+\['shift'\]\(\)\)|\.push\(\w+\.shift\(\)\)/.test(code) ? "push-shift" : "-",
  };
}

function runtimeOf(output) {
  const head = output.slice(0, 400000);
  if (/void 0 ?!== ?\w+\)\s*\{?\s*return \w+\.exports/.test(head) || /__webpack_module_cache__/.test(head)) return "webpack5";
  if (/\.call\(\w+\.exports, ?\w+, ?\w+\.exports, ?\w+\)/.test(head)) return "webpack4";
  return "none";
}

function helpersOf(output) {
  const head = output.slice(0, 400000);
  const counts = new Map();
  for (const match of head.matchAll(/\b([A-Za-z_$][\w$]*)\.([a-z])\s*=\s*(?:function|\(|[A-Za-z_$])/g)) {
    const key = match[1];
    if (!counts.has(key)) counts.set(key, new Set());
    counts.get(key).add(match[2]);
  }
  const best = [...counts.values()].sort((a, b) => b.size - a.size)[0];
  return best && best.size >= 3 ? [...best].sort().join("") : "-";
}

function noteFeatures(stderr) {
  const notes = stderr.split("\n").filter((line) => line.startsWith("note: "));
  const decoders = /string array decoders found: (\d+)/.exec(stderr);
  const entries = /string array found: (\d+) entries/.exec(stderr);
  return {
    decoders: decoders ? decoders[1] : "0",
    entries: bucket(entries ? Number(entries[1]) : 0),
    shell: notes.some((n) => n.includes("strip-shell")) ? "shell" : "-",
    bundle: (/^bundle: (\w+)/m.exec(stderr) ?? [])[1] ?? "none",
  };
}

function describe(dir, name) {
  const match = NAME.exec(name);
  if (!match) return undefined;
  const version = match[3];
  return { name, file: join(dir, name), kind: match[1], version, minor: version.split(".").slice(0, 2).join("."), bytes: statSync(join(dir, name)).size };
}

async function pool(items, jobs, work) {
  const queue = [...items];
  await Promise.all(Array.from({ length: jobs }, async () => {
    while (queue.length > 0) await work(queue.shift());
  }));
}

function versionKey(version) {
  return version.split(/[.r]/).map((part) => part.padStart(4, "0")).join(".");
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  mkdirSync(options.out, { recursive: true });
  const entries = readdirSync(options.export).sort().map((name) => describe(options.export, name)).filter(Boolean);
  const rows = [];
  await pool(entries, options.jobs, async (entry) => {
    const code = readFileSync(entry.file, "utf8");
    const result = await run(options.untwist, ["--unsafe", "--report", "text", entry.file]);
    const features = { kind: entry.kind, ...inputFeatures(code), ...noteFeatures(result.stderr), runtime: runtimeOf(result.stdout), helpers: helpersOf(result.stdout) };
    const signature = Object.values(features).join(" ");
    rows.push({ ...entry, status: result.status, signature });
    process.stdout.write(`${entry.name.slice(0, 24)} ${signature}\n`);
  });
  rows.sort((a, b) => a.name.localeCompare(b.name));
  writeFileSync(join(options.out, "signatures.tsv"), rows.map((r) => [r.name, r.kind, r.version, r.bytes, r.status, r.signature].join("\t")).join("\n") + "\n");
  const clusters = new Map();
  for (const row of rows) {
    if (!clusters.has(row.signature)) clusters.set(row.signature, []);
    clusters.get(row.signature).push(row);
  }
  const lines = [...clusters.entries()].map(([signature, members]) => {
    const versions = members.map((m) => m.version).sort((a, b) => versionKey(a).localeCompare(versionKey(b)));
    const representative = [...members].sort((a, b) => a.bytes - b.bytes)[0];
    return [members.length, versions[0], versions.at(-1), representative.name, signature].join("\t");
  });
  writeFileSync(join(options.out, "clusters.tsv"), `# files\tfrom\tto\trepresentative\tsignature\n${lines.join("\n")}\n`);
  process.stdout.write(`cluster: ${rows.length} files, ${clusters.size} clusters\n`);
}

await main();
