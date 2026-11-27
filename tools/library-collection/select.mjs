import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { ascending, compareVersions, isRelease, parseArgs } from "./common.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const BULK = 128;
const DOWNLOADS = "https://api.npmjs.org/downloads/point/last-month/";
const VERSIONS = "https://api.npmjs.org/versions/";
const REGISTRY = "https://registry.npmjs.org/";
const COVERAGE = 0.8;
const MINOR_RANK = 100;
const PAUSE = 300;
const MAJOR_SHARE = 0.02;

const options = parseArgs(process.argv.slice(2), { top: "200", "max-versions": "5" });
if (!options.out) {
  console.error("usage: node select.mjs --out <dir> [--top 200] [--max-versions 5] [--wanted <file>] [--snapshot <file>]");
  console.error("--wanted: every name in the file is sent to api.npmjs.org and registry.npmjs.org; give only public package names, never a list taken from a private bundle");
  process.exit(2);
}
const overrides = JSON.parse(readFileSync(join(HERE, "overrides.json"), "utf8"));
const lines = (file) =>
  readFileSync(file, "utf8")
    .split("\n")
    .map((line) => line.replace(/#.*/, "").trim())
    .filter(Boolean);

async function fetchJson(url) {
  for (let attempt = 0; attempt < 6; attempt++) {
    const response = await fetch(url, { headers: { "user-agent": "untwist-library-collection" } });
    if (response.status === 404) return null;
    if (response.ok) return response.json();
    await new Promise((done) => setTimeout(done, 5000 * (attempt + 1)));
  }
  console.error(`${url}: no answer, skipped`);
  return null;
}

async function downloads(names) {
  const out = {};
  const plain = names.filter((name) => !name.startsWith("@"));
  for (let at = 0; at < plain.length; at += BULK) {
    const chunk = plain.slice(at, at + BULK);
    const data = await fetchJson(DOWNLOADS + chunk.join(","));
    if (data === null) continue;
    if (chunk.length === 1) out[chunk[0]] = data.downloads ?? 0;
    else for (const name of chunk) out[name] = data[name]?.downloads ?? 0;
  }
  for (const name of names.filter((name) => name.startsWith("@"))) {
    const data = await fetchJson(DOWNLOADS + name);
    if (data !== null) out[name] = data.downloads ?? 0;
    await new Promise((done) => setTimeout(done, PAUSE));
  }
  return out;
}

function bundleable(manifest) {
  if (!manifest || manifest.deprecated) return false;
  if (manifest.name.startsWith("@types/")) return false;
  const entry = manifest.browser || manifest.module || manifest.exports || manifest.main;
  return Boolean(entry) || !manifest.bin;
}

async function snapshot(names) {
  const counts = await downloads(names);
  const ranked = names.filter((name) => counts[name] > 0).sort((a, b) => counts[b] - counts[a] || a.localeCompare(b));
  const packages = {};
  for (const name of ranked) {
    const latest = await fetchJson(`${REGISTRY}${name.replace("/", "%2f")}/latest`);
    const perVersion = (await fetchJson(`${VERSIONS}${encodeURIComponent(name)}/last-week`))?.downloads ?? {};
    packages[name] = {
      downloads: counts[name],
      latest: latest?.version ?? null,
      bundleable: bundleable(latest),
      versions: perVersion,
    };
    console.error(`${name}: ${counts[name]}`);
    await new Promise((done) => setTimeout(done, PAUSE));
  }
  const missing = names.filter((name) => counts[name] === 0);
  return { taken: new Date().toISOString().slice(0, 10), packages, missing };
}

function pickVersions(name, info, rank, limit) {
  const fixed = overrides.versions?.[name];
  if (fixed) return ascending(fixed);
  const entries = Object.entries(info.versions).filter(([version]) => isRelease(version));
  const total = entries.reduce((sum, [, count]) => sum + count, 0);
  const picked = new Set();
  if (info.latest && isRelease(info.latest)) picked.add(info.latest);
  let covered = 0;
  for (const [version, count] of [...entries].sort((a, b) => b[1] - a[1] || compareVersions(b[0], a[0]))) {
    if (total === 0 || covered / total >= COVERAGE) break;
    picked.add(version);
    covered += count;
  }
  const group = (version) => {
    const [major, minor] = version.split(".");
    return rank < MINOR_RANK && major === "0" ? `${major}.${minor}` : major;
  };
  const newest = {};
  for (const [version] of entries) {
    const key = group(version);
    if (!newest[key] || compareVersions(version, newest[key]) > 0) newest[key] = version;
  }
  const weight = (version) => info.versions[version] ?? 0;
  const groupWeight = {};
  for (const [version, count] of entries) groupWeight[group(version)] = (groupWeight[group(version)] ?? 0) + count;
  const majors = Object.entries(newest)
    .filter(([key]) => total > 0 && groupWeight[key] >= total * MAJOR_SHARE)
    .sort((a, b) => groupWeight[b[0]] - groupWeight[a[0]] || compareVersions(b[1], a[1]))
    .map(([, version]) => version);
  const chosen = [...picked].sort((a, b) => weight(b) - weight(a) || compareVersions(b, a));
  const merged = [];
  for (const version of [...(info.latest && isRelease(info.latest) ? [info.latest] : []), ...chosen, ...majors]) {
    if (!merged.includes(version)) merged.push(version);
  }
  return ascending(merged.slice(0, limit));
}

const names = [...new Set([...lines(join(HERE, "candidates.txt")), ...(options.wanted ? lines(options.wanted) : [])])];
mkdirSync(options.out, { recursive: true });
const snapshotFile = options.snapshot ?? join(options.out, "npm-snapshot.json");
let data;
if (existsSync(snapshotFile)) {
  data = JSON.parse(readFileSync(snapshotFile, "utf8"));
  const missing = names.filter((name) => !(name in data.packages) && !(data.missing ?? []).includes(name));
  if (missing.length) {
    const extra = await snapshot(missing);
    Object.assign(data.packages, extra.packages);
    data.missing = [...(data.missing ?? []), ...extra.missing];
  }
} else {
  data = await snapshot(names);
}
writeFileSync(snapshotFile, JSON.stringify(data, null, 1) + "\n");

const skip = new Set(overrides.skip ?? []);
const top = Number(options.top);
const limit = Number(options["max-versions"]);
const eligible = Object.entries(data.packages)
  .filter(([name, info]) => names.includes(name) && info.bundleable && info.latest && !skip.has(name))
  .sort((a, b) => b[1].downloads - a[1].downloads || a[0].localeCompare(b[0]));
const always = (overrides.always ?? []).filter((name) => data.packages[name]?.latest);
const chosen = [...new Set([...eligible.slice(0, top).map(([name]) => name), ...always])];
const selection = chosen.map((name, rank) => ({
  name,
  downloads: data.packages[name].downloads,
  versions: pickVersions(name, data.packages[name], rank, limit),
}));
writeFileSync(join(options.out, "selection.json"), JSON.stringify({ taken: data.taken, packages: selection }, null, 1) + "\n");
console.error(`${selection.length} packages, ${selection.reduce((sum, item) => sum + item.versions.length, 0)} versions`);
