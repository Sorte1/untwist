import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { CAUSES, causeOf } from "./causes.mjs";


const LANES = ["default", "unsafe", "reference"];

function loadRows(dir) {
  return readdirSync(dir)
    .filter((name) => /^results\..*\.json$/.test(name))
    .flatMap((name) => JSON.parse(readFileSync(join(dir, name), "utf8")));
}

function core(row, lane) {
  return row.residue[lane]?.core ?? null;
}

function shortId(id) {
  return id.replaceAll("__", "/");
}

function execMark(row, lane) {
  if (!row.sameOutput) return "";
  return row.sameOutput[lane] ? "" : " ✗";
}

function cell(row, lane) {
  const value = core(row, lane);
  if (value === null) return "n/a";
  return `${value}${execMark(row, lane)}`;
}

function verdict(row) {
  const [d, u, w] = LANES.map((lane) => core(row, lane));
  if (d === null || u === null || w === null) return "unmeasured";
  if (d > w || u > w) return d > w && u > w ? "untwist worse (both)" : d > w ? "untwist worse (default)" : "untwist worse (unsafe)";
  if (w > Math.min(d, u)) return "reference worse";
  return "tie";
}

function group(id) {
  const parts = id.split("__");
  return parts.length > 1 ? parts.slice(0, -1).join("/") : "repo root";
}

function inputTables(rows) {
  const groups = new Map();
  for (const row of rows) groups.set(group(row.id), [...(groups.get(group(row.id)) ?? []), row]);
  const out = [];
  for (const [name, members] of groups) {
    out.push(`### ${name}`, "", "| input | input markers | untwist default | untwist --unsafe | reference | verdict |", "|---|---:|---:|---:|---:|---|");
    for (const row of members) {
      out.push(`| ${row.id.split("__").pop()} | ${core(row, "input") ?? "n/a"} | ${cell(row, "default")} | ${cell(row, "unsafe")} | ${cell(row, "reference")} | ${verdict(row)} |`);
    }
    out.push("");
  }
  return out;
}

function worseRows(rows) {
  return rows.filter((row) => verdict(row).startsWith("untwist worse"));
}

function gapEntry(row) {
  const lane = core(row, "default") > core(row, "reference") ? "default" : "unsafe";
  const left = core(row, lane) - core(row, "reference");
  const notes = row.notes[lane].filter((note) => !/string array found|rotator found|decoders found/.test(note));
  const named = causeOf(notes);
  const cause = named === "unexplained" && lane === "default" && core(row, "unsafe") <= core(row, "reference") ? "unsafe-only" : named;
  return { row, lane, left, cause, notes };
}

function causeTable(entries) {
  const byCause = new Map();
  for (const entry of entries) byCause.set(entry.cause, [...(byCause.get(entry.cause) ?? []), entry]);
  const ranked = [...byCause.entries()].sort((a, b) => sum(b[1]) - sum(a[1]));
  const label = (key) => CAUSES.find(([name]) => name === key)?.[1] ?? "no refusal note names the cause";
  const out = ["| rank | cause | markers left beyond reference | inputs | reference output runs the same | inputs affected |", "|---:|---|---:|---:|---:|---|"];
  ranked.forEach(([key, members], index) => {
    const same = members.filter((m) => m.row.sameOutput?.reference).length;
    const checked = members.filter((m) => m.row.sameOutput).length;
    const names = members.sort((a, b) => b.left - a.left).map((m) => m.row.id.split("__").pop());
    const shown = names.length > 8 ? `${names.slice(0, 8).join(", ")}, … (+${names.length - 8})` : names.join(", ");
    out.push(`| ${index + 1} | \`${key}\`: ${label(key)} | ${sum(members)} | ${members.length} | ${checked ? `${same}/${checked}` : "n/a"} | ${shown} |`);
  });
  return out;
}

function sum(entries) {
  return entries.reduce((total, entry) => total + entry.left, 0);
}

function noteList(entries) {
  const out = [];
  for (const entry of entries.sort((a, b) => b.left - a.left)) {
    const wcRuns = entry.row.sameOutput ? (entry.row.sameOutput.reference ? "reference output runs the same" : "reference output runs differently") : "not executed";
    out.push(`- **${shortId(entry.row.id)}** (${entry.lane}, +${entry.left}, \`${entry.cause}\`, ${wcRuns})`);
    for (const note of [...new Set(entry.notes)].slice(0, 4)) out.push(`  - \`${note.replaceAll("`", "'").slice(0, 300)}\``);
  }
  return out;
}

function main() {
  const dir = process.argv[2];
  const rows = loadRows(dir);
  const entries = worseRows(rows).map(gapEntry);
  const section = process.argv[3] ?? "all";
  const parts = {
    tables: inputTables(rows),
    causes: causeTable(entries),
    notes: noteList(entries),
    tsv: [["input", "input_markers", "default", "unsafe", "reference", "verdict", "reference_runs_same", "cause"].join("\t"), ...rows.map((row) => {
      const worse = verdict(row).startsWith("untwist worse");
      return [shortId(row.id), core(row, "input") ?? "", core(row, "default") ?? "", core(row, "unsafe") ?? "", core(row, "reference") ?? "", verdict(row), row.sameOutput ? String(row.sameOutput.reference) : "", worse ? gapEntry(row).cause : ""].join("\t");
    })],
    reference: rows.filter((row) => verdict(row) === "reference worse").map((row) => `- ${shortId(row.id)}: input ${core(row, "input")}, default ${cell(row, "default")}, unsafe ${cell(row, "unsafe")}, reference ${cell(row, "reference")}`),
  };
  const chosen = section === "all" ? [...parts.tables, ...parts.causes, ...parts.notes, ...parts.reference] : parts[section];
  process.stdout.write(`${chosen.join("\n")}\n`);
}

main();
