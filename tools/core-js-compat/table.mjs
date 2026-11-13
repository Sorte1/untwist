import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = dirname(fileURLToPath(import.meta.url));
const MODULES = join(ROOT, "node_modules");

const YEAR_ENGINES = {
  chrome: "chrome",
  edge: "edge",
  firefox: "firefox",
  safari: "safari",
  safari_ios: "ios",
  nodejs: "node",
  opera: "opera",
  samsunginternet_android: "samsung",
};

const FEATURES = {
  2015: [
    "javascript.functions.arrow_functions",
    "javascript.classes",
    "javascript.statements.let",
    "javascript.statements.const",
    "javascript.operators.destructuring",
    "javascript.functions.default_parameters",
    "javascript.functions.rest_parameters",
    "javascript.operators.spread.spread_in_function_calls",
    "javascript.grammar.template_literals",
    "javascript.statements.for_of",
    "javascript.statements.generator_function",
    "javascript.operators.new_target",
    "javascript.builtins.Symbol",
    "javascript.builtins.Promise",
    "javascript.builtins.Map",
    "javascript.builtins.Set",
    "javascript.builtins.WeakMap",
    "javascript.builtins.WeakSet",
    "javascript.builtins.Proxy",
    "javascript.builtins.Reflect",
    "javascript.builtins.Array.from",
    "javascript.builtins.Array.find",
    "javascript.builtins.Object.assign",
    "javascript.builtins.String.includes",
    "javascript.builtins.String.raw",
    "javascript.builtins.Number.isInteger",
    "javascript.builtins.Math.trunc",
  ],
  2016: ["javascript.operators.exponentiation", "javascript.builtins.Array.includes"],
  2017: [
    "javascript.statements.async_function",
    "javascript.builtins.Object.values",
    "javascript.builtins.Object.entries",
    "javascript.builtins.Object.getOwnPropertyDescriptors",
    "javascript.builtins.String.padStart",
    "javascript.builtins.String.padEnd",
  ],
  2018: [
    "javascript.statements.async_generator_function",
    "javascript.statements.for_await_of",
    "javascript.operators.spread.spread_in_object_literals",
    "javascript.operators.destructuring.rest_in_objects",
    "javascript.builtins.RegExp.dotAll",
    "javascript.regular_expressions.named_capturing_group",
    "javascript.regular_expressions.lookbehind_assertion",
    "javascript.regular_expressions.unicode_character_class_escape",
    "javascript.builtins.Promise.finally",
    "javascript.builtins.Symbol.asyncIterator",
  ],
  2019: [
    "javascript.builtins.Array.flat",
    "javascript.builtins.Array.flatMap",
    "javascript.builtins.Object.fromEntries",
    "javascript.builtins.String.trimStart",
    "javascript.builtins.String.trimEnd",
    "javascript.builtins.Symbol.description",
    "javascript.statements.try_catch.optional_catch_binding",
  ],
  2020: [
    "javascript.operators.optional_chaining",
    "javascript.operators.nullish_coalescing",
    "javascript.builtins.BigInt",
    "javascript.builtins.globalThis",
    "javascript.builtins.Promise.allSettled",
    "javascript.builtins.String.matchAll",
    "javascript.statements.export.namespace",
  ],
  2021: [
    "javascript.operators.logical_or_assignment",
    "javascript.operators.nullish_coalescing_assignment",
    "javascript.grammar.numeric_separators",
    "javascript.builtins.String.replaceAll",
    "javascript.builtins.Promise.any",
    "javascript.builtins.AggregateError",
    "javascript.builtins.WeakRef",
    "javascript.builtins.FinalizationRegistry",
  ],
  2022: [
    "javascript.classes.public_class_fields",
    "javascript.classes.private_class_fields",
    "javascript.classes.private_class_methods",
    "javascript.classes.static.initialization_blocks",
    "javascript.classes.private_class_fields_in",
    "javascript.builtins.Array.at",
    "javascript.builtins.String.at",
    "javascript.builtins.Object.hasOwn",
    "javascript.builtins.Error.cause",
    "javascript.builtins.RegExp.hasIndices",
    "javascript.operators.await.top_level",
  ],
  2023: [
    "javascript.builtins.Array.findLast",
    "javascript.builtins.Array.findLastIndex",
    "javascript.builtins.Array.toSorted",
    "javascript.builtins.Array.toReversed",
    "javascript.builtins.Array.toSpliced",
    "javascript.builtins.Array.with",
    "javascript.grammar.hashbang_comments",
  ],
  2024: [
    "javascript.builtins.Object.groupBy",
    "javascript.builtins.Map.groupBy",
    "javascript.builtins.Promise.withResolvers",
    "javascript.builtins.String.isWellFormed",
    "javascript.builtins.String.toWellFormed",
    "javascript.builtins.ArrayBuffer.transfer",
    "javascript.builtins.ArrayBuffer.resizable",
    "javascript.builtins.Atomics.waitAsync",
    "javascript.builtins.RegExp.unicodeSets",
  ],
};

const FIRST_YEAR = 2015;
const LAST_YEAR = 2024;
const OFFERED_FROM = 2017;

function key(version) {
  return String(version).split(/[.\-+]/).map((part) => Number.parseInt(part, 10) || 0);
}

function compare(a, b) {
  const [x, y] = [key(a), key(b)];
  for (let i = 0; i < Math.max(x.length, y.length); i++) {
    const d = (x[i] ?? 0) - (y[i] ?? 0);
    if (d !== 0) return d;
  }
  return 0;
}

function maxVersion(a, b) {
  if (a === undefined) return b;
  return compare(a, b) >= 0 ? a : b;
}

function readJson(...parts) {
  return JSON.parse(readFileSync(join(MODULES, ...parts), "utf8"));
}

function compatVersions() {
  return readdirSync(MODULES)
    .filter((dir) => /^core-js-compat-3\.\d+$/.test(dir))
    .map((dir) => ({ dir, version: readJson(dir, "package.json").version }))
    .sort((a, b) => compare(a.version, b.version));
}

function ranges(indices, versions) {
  const out = [];
  let start = null;
  let previous = null;
  for (const index of indices) {
    if (start !== null && index === previous + 1) {
      previous = index;
      continue;
    }
    if (start !== null) out.push([start, previous]);
    start = index;
    previous = index;
  }
  if (start !== null) out.push([start, previous]);
  return out
    .map(([low, high]) => (low === high ? versions[low] : `${versions[low]}~${versions[high]}`))
    .join(",");
}

function firstShipped(entries) {
  const list = Array.isArray(entries) ? entries : [entries];
  const full = list.filter(
    (entry) =>
      entry &&
      typeof entry.version_added === "string" &&
      /^[\u2264]?\d/.test(entry.version_added) &&
      !entry.flags &&
      !entry.prefix &&
      !entry.alternative_name,
  );
  const versions = full.map((entry) => entry.version_added.replace("\u2264", ""));
  return versions.sort(compare)[0];
}

function feature(bcd, path) {
  const node = path.split(".").reduce((object, key) => object && object[key], bcd);
  if (!node?.__compat) throw new Error(`@mdn/browser-compat-data has no ${path}`);
  return node.__compat.support;
}

function yearBaselines(bcd) {
  const lines = [];
  const need = {};
  for (let year = FIRST_YEAR; year <= LAST_YEAR; year++) {
    for (const path of FEATURES[year] ?? []) {
      const support = feature(bcd, path);
      for (const [browser, engine] of Object.entries(YEAR_ENGINES)) {
        const version = firstShipped(support[browser]);
        if (!version) throw new Error(`${path} has no shipped ${browser} version`);
        need[engine] = maxVersion(need[engine], version);
      }
    }
    if (year >= OFFERED_FROM) {
      const engines = Object.values(YEAR_ENGINES).map((engine) => `${engine}:${need[engine]}`).join(",");
      lines.push(`#baseline\tes${year}\t${engines}`);
    }
  }
  return lines;
}

function main() {
  const compat = compatVersions();
  const versions = compat.map((entry) => entry.version);
  const rows = new Map();
  const datasets = compat.map((entry) => readJson(entry.dir, "data.json"));
  datasets.forEach((data, index) => {
    for (const [module, engines] of Object.entries(data)) {
      for (const [engine, version] of Object.entries(engines)) {
        const id = `${module}\t${engine}\t${version}`;
        if (!rows.has(id)) rows.set(id, []);
        rows.get(id).push(index);
      }
    }
  });
  const bcd = readJson("@mdn", "browser-compat-data", "data.json");
  const mdn = readJson("@mdn", "browser-compat-data", "package.json").version;
  const out = ["#untwist-core-js-compat\tv1"];
  out.push(`#source\tcore-js-compat ${versions[0]}~${versions[versions.length - 1]}, @mdn/browser-compat-data ${mdn}`);
  out.push(`#versions\t${versions.join(",")}`);
  out.push(...yearBaselines(bcd));
  const body = [...rows.entries()]
    .map(([id, indices]) => `${id}\t${ranges(indices, versions)}`)
    .sort();
  out.push(...body);
  process.stdout.write(`${out.join("\n")}\n`);
}

main();
