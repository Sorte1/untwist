import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { causeInFamily, causeLabel, familyCounts } from "./causes.mjs";

const MARKER_KEYS = ["stringArrays", "rotators", "decoderCalls", "arrayIndex", "cffSwitches", "opaquePredicates", "proxyObjects", "evalCalls", "functionCtors"];

function load(dir, mode) {
  const suffix = `.${mode}.result.json`;
  return readdirSync(dir)
    .filter((name) => name.endsWith(suffix))
    .map((name) => JSON.parse(readFileSync(join(dir, name), "utf8")));
}

function core(row) {
  if (!row || row.error || row.core === null || row.core === undefined) return null;
  return row.core;
}

function parses(row) {
  return Boolean(row) && !row.error && !row.parseError && (row.parseErrors ?? 0) === 0;
}

function tsv(untwist, reference) {
  const lines = [["file", "kind", "version", "lane", "markers", "input_markers", "decoder_calls", "opaque_predicates", "wall_s", "peak_rss_mb", "exit", "parses"].join("\t")];
  const short = (entry) => `${entry.kind}-${entry.sha}-${entry.version}.js`;
  for (const entry of [...untwist].sort((a, b) => a.name.localeCompare(b.name))) {
    for (const lane of ["default", "unsafe"]) {
      const run = entry.untwist[lane];
      const residue = entry.residue[lane];
      lines.push([short(entry), entry.kind, entry.version, `untwist-${lane}`, core(residue) ?? "", core(entry.input) ?? "", residue?.decoderCalls ?? "", residue?.opaquePredicates ?? "", run.wallS, Math.round(run.rssKb / 1024), run.timedOut ? "timeout" : run.status, parses(residue)].join("\t"));
    }
  }
  for (const entry of [...reference].sort((a, b) => a.name.localeCompare(b.name))) {
    const residue = entry.residue.reference;
    lines.push([short(entry), entry.kind, entry.version, "reference", core(residue) ?? "", "", residue?.decoderCalls ?? "", residue?.opaquePredicates ?? "", entry.reference.wallS, Math.round(entry.reference.rssKb / 1024), entry.reference.timedOut ? "timeout" : entry.reference.status, parses(residue)].join("\t"));
  }
  return `${lines.join("\n")}\n`;
}

function minorOf(version) {
  return version.split(".").slice(0, 2).join(".");
}

function compareMinor(a, b) {
  const [a1, a2] = a.split(".").map(Number);
  const [b1, b2] = b.split(".").map(Number);
  return a1 - b1 || a2 - b2;
}

function totals(rows, pick) {
  const values = rows.map(pick).filter((value) => value !== null && value !== undefined);
  return values.length ? values.reduce((sum, value) => sum + value, 0) : null;
}

function versionTable(untwist, reference) {
  const out = ["| version | kind | files | input markers | untwist default | untwist --unsafe | reference sample (files) | untwist default on the same sample | failures (crash/timeout/unparsable) | median default wall s | max RSS MB |", "|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|"];
  const keys = [...new Set(untwist.map((entry) => `${minorOf(entry.version)}|${entry.kind}`))].sort((a, b) => compareMinor(a.split("|")[0], b.split("|")[0]) || a.localeCompare(b));
  for (const key of keys) {
    const [minor, kind] = key.split("|");
    const rows = untwist.filter((entry) => minorOf(entry.version) === minor && entry.kind === kind);
    const sampled = reference.filter((entry) => minorOf(entry.version) === minor && entry.kind === kind);
    const sampledNames = new Set(sampled.map((entry) => entry.name));
    const failures = rows.filter((entry) => ["default", "unsafe"].some((lane) => entry.untwist[lane].status !== 0 || !parses(entry.residue[lane]))).length;
    const walls = rows.map((entry) => entry.untwist.default.wallS).sort((a, b) => a - b);
    const rss = Math.max(...rows.flatMap((entry) => ["default", "unsafe"].map((lane) => entry.untwist[lane].rssKb / 1024)));
    const wc = totals(sampled, (entry) => core(entry.residue.reference));
    const same = totals(rows.filter((entry) => sampledNames.has(entry.name)), (entry) => core(entry.residue.default));
    out.push(`| ${minor} | ${kind} | ${rows.length} | ${totals(rows, (e) => core(e.input))} | ${totals(rows, (e) => core(e.residue.default))} | ${totals(rows, (e) => core(e.residue.unsafe))} | ${sampled.length ? `${wc ?? "n/a"} (${sampled.length})` : "—"} | ${sampled.length ? same ?? "n/a" : "—"} | ${failures} | ${walls[Math.floor(walls.length / 2)]} | ${Math.round(rss)} |`);
  }
  return out;
}

function markerBreakdown(untwist) {
  const out = ["| version | kind | " + MARKER_KEYS.join(" | ") + " |", `|---|---|${MARKER_KEYS.map(() => "---:").join("|")}|`];
  const keys = [...new Set(untwist.map((entry) => `${minorOf(entry.version)}|${entry.kind}`))].sort((a, b) => compareMinor(a.split("|")[0], b.split("|")[0]) || a.localeCompare(b));
  for (const key of keys) {
    const [minor, kind] = key.split("|");
    const rows = untwist.filter((entry) => minorOf(entry.version) === minor && entry.kind === kind);
    out.push(`| ${minor} | ${kind} | ${MARKER_KEYS.map((marker) => `${totals(rows, (e) => e.residue.default?.[marker] ?? null)} / ${totals(rows, (e) => e.residue.unsafe?.[marker] ?? null)}`).join(" | ")} |`);
  }
  return out;
}

function addTo(groups, key, fields, left, entry) {
  const group = groups.get(key) ?? { ...fields, left: 0, files: new Set(), versions: new Set() };
  group.left += left;
  group.files.add(entry.name);
  group.versions.add(minorOf(entry.version) + (entry.kind === "loader" ? "L" : ""));
  groups.set(key, group);
}

function failureCause(run) {
  if (run.timedOut) return "timeout";
  if (run.status === 1 && run.notes.some((note) => /no string array found/.test(note))) return "no-string-array";
  return run.status !== 0 ? `exit-${run.status}` : "unparsable-output";
}

function laneCause(run, lane, family, unsafeFamilies) {
  const named = causeInFamily(run.notes, family);
  const clearsUnderUnsafe = lane === "default" && unsafeFamilies[family] === 0;
  return named.startsWith("no-note:") && clearsUnderUnsafe ? "unsafe-only" : named;
}

function residueGroups(untwist) {
  const groups = new Map();
  for (const entry of untwist) {
    for (const lane of ["default", "unsafe"]) {
      const run = entry.untwist[lane];
      const residue = entry.residue[lane];
      if (run.status !== 0 || !parses(residue)) {
        const cause = failureCause(run);
        addTo(groups, `${cause}|${lane}`, { cause, lane }, core(entry.input) ?? 0, entry);
        continue;
      }
      const unsafeFamilies = familyCounts(entry.residue.unsafe);
      for (const [family, left] of Object.entries(familyCounts(residue))) {
        if (left === 0) continue;
        const cause = laneCause(run, lane, family, unsafeFamilies);
        addTo(groups, `${cause}|${lane}`, { cause, lane }, left, entry);
      }
    }
  }
  return groups;
}

function baselines(reference) {
  const byGroup = new Map();
  for (const sampled of reference) {
    if (!parses(sampled.residue.reference)) continue;
    const key = `${minorOf(sampled.version)}|${sampled.kind}`;
    const families = familyCounts(sampled.residue.reference);
    const current = byGroup.get(key) ?? {};
    byGroup.set(key, Object.fromEntries(Object.entries(families).map(([family, count]) => [family, Math.max(count, current[family] ?? 0)])));
  }
  return byGroup;
}

function laneGaps(entry, wcFamilies) {
  const found = [];
  for (const lane of ["default", "unsafe"]) {
    const run = entry.untwist[lane];
    const residue = entry.residue[lane];
    const failed = run.status !== 0 || !parses(residue);
    const ours = failed ? familyCounts(entry.input) : familyCounts(residue);
    for (const [family, left] of Object.entries(ours)) {
      const beyond = left - wcFamilies[family];
      if (beyond <= 0) continue;
      const cause = failed ? failureCause(run) : laneCause(run, lane, family, familyCounts(entry.residue.unsafe));
      found.push({ lane, family, beyond, cause });
    }
  }
  return found;
}

function gapGroups(untwist, reference, estimate) {
  const measured = new Map(reference.filter((sampled) => parses(sampled.residue.reference)).map((sampled) => [sampled.name, familyCounts(sampled.residue.reference)]));
  const baseline = baselines(reference);
  const groups = new Map();
  const rows = [];
  for (const entry of untwist) {
    const exact = measured.get(entry.name);
    const wcFamilies = exact ?? (estimate ? baseline.get(`${minorOf(entry.version)}|${entry.kind}`) : undefined);
    if (!wcFamilies) continue;
    const worst = new Map();
    for (const gap of laneGaps(entry, wcFamilies)) {
      rows.push({ name: entry.name, version: entry.version, kind: entry.kind, reference: exact ? "measured" : "minor-baseline", ...gap });
      if ((worst.get(gap.family)?.beyond ?? 0) < gap.beyond) worst.set(gap.family, gap);
    }
    for (const { beyond, cause } of worst.values()) addTo(groups, cause, { cause }, beyond, entry);
  }
  return { groups, rows };
}

function rankedTable(groups, withLane) {
  const ranked = [...groups.values()].sort((a, b) => b.left - a.left);
  const header = withLane ? "| rank | cause | lane | markers left | files | versions (L = loader) |" : "| rank | cause | markers beyond reference | inputs | versions (L = loader) |";
  const out = [header, withLane ? "|---:|---|---|---:|---:|---|" : "|---:|---|---:|---:|---|"];
  ranked.forEach((group, index) => {
    const cells = [index + 1, `\`${group.cause}\`: ${causeLabel(group.cause)}`, ...(withLane ? [group.lane] : []), group.left, group.files.size, [...group.versions].sort(compareMinor).join(", ")];
    out.push(`| ${cells.join(" | ")} |`);
  });
  return out;
}

function gapTsv(rows) {
  const lines = [["file", "kind", "version", "lane", "family", "markers_beyond_reference", "reference", "cause"].join("\t")];
  for (const row of rows) lines.push([row.name.replace(/-([0-9a-f]{12})[0-9a-f]+-/, "-$1-"), row.kind, row.version, row.lane, row.family, row.beyond, row.reference, row.cause].join("\t"));
  return `${lines.join("\n")}\n`;
}

function main() {
  const [dir, tsvPath, section, gapPath, referenceDir = dir] = process.argv.slice(2);
  const untwist = load(dir, "untwist");
  const reference = load(referenceDir, "reference");
  if (tsvPath) writeFileSync(tsvPath, tsv(untwist, reference));
  const gaps = gapGroups(untwist, reference, false);
  const estimated = gapGroups(untwist, reference, true);
  if (gapPath) writeFileSync(gapPath, gapTsv(estimated.rows));
  const parts = { versions: versionTable(untwist, reference), markers: markerBreakdown(untwist), causes: rankedTable(residueGroups(untwist), true), gaps: rankedTable(gaps.groups, false), estimated: rankedTable(estimated.groups, false) };
  const chosen = !section || section === "all" ? Object.values(parts).flat() : parts[section];
  process.stdout.write(`${chosen.join("\n")}\n`);
}

main();
